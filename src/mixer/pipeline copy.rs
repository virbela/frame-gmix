use glib::translate::FromGlib;
use gstreamer::traits::ElementExt;
use gstreamer::{element_error, prelude::*};
use gstreamer::{ElementFactory, Pipeline};
use std::error::Error;
use std::sync::{Arc, Mutex};

pub struct AudioMixerPipeline {
    pipeline: Arc<Mutex<Option<Pipeline>>>,
    input_ports: Vec<u16>,
}

impl AudioMixerPipeline {
    pub fn new(
        input_ports: Vec<u16>,
        destination_ip: &str,
        destination_port: u16,
    ) -> Result<Self, Box<dyn Error>> {
        gstreamer::init()?;

        let mut input_elements = Vec::new();

        for port in &input_ports {
            let udpsrc = ElementFactory::make("udpsrc")
                .build()
                .expect("failed to create udpsrc");
            udpsrc.set_property_from_str("port", &port.to_string());

            let rtpbin = ElementFactory::make("rtpbin")
                .build()
                .expect("failed to create rtpbin");

            let depay = ElementFactory::make("rtpopusdepay")
                .build()
                .expect("failed to create rtpopusdepay");
            let parse = ElementFactory::make("opusparse")
                .build()
                .expect("failed to create opusparse");
            let dec = ElementFactory::make("opusdec")
                .build()
                .expect("failed to create opusdec");
            let conv = ElementFactory::make("audioconvert")
                .build()
                .expect("failed to create audioconvert");
            let elements = (udpsrc, rtpbin.clone(), depay.clone(), parse, dec, conv);
            input_elements.push(elements);
        }

        let audiomixer = ElementFactory::make("audiomixer")
            .build()
            .expect("failed to create audiomixer");
        let opusenc = ElementFactory::make("opusenc")
            .build()
            .expect("failed to create opusenc");
        let rtpopuspay = ElementFactory::make("rtpopuspay")
            .build()
            .expect("failed to create rtpopuspay");
        let udpsink = ElementFactory::make("udpsink")
            .build()
            .expect("failed to create udpsink");

        udpsink.set_property_from_str("host", destination_ip);
        udpsink.set_property_from_str("port", &destination_port.to_string());

        let pipeline = Pipeline::new(None);

        for (i, (udpsrc, rtpbin, depay, parse, dec, conv)) in input_elements.into_iter().enumerate()
        {
            pipeline.add_many(&[&udpsrc, &rtpbin, &depay, &parse, &dec, &conv])?;

            let depay_clone = depay.clone();
            let audiomixer_clone = audiomixer.clone();
            let conv_clone = conv.clone();

            let _ = rtpbin.connect("pad-added", false, move |args| {
                let new_pad = match args[1].get::<gstreamer::Pad>() {
                    Ok(pad) => pad,
                    Err(_) => {
                        eprintln!("Failed to get Pad from args");
                        return None;
                    }
                };

                let new_pad_name = new_pad.name();
                if new_pad_name.starts_with("recv_rtp_src_0") {
                    let depay_sink_pad = depay_clone
                        .static_pad("sink")
                        .expect("Failed to get sink pad from rtpopusdepay");
                    new_pad
                        .link(&depay_sink_pad)
                        .expect("Failed to link rtpbin and rtpopusdepay");

                    let sinkpad_template = audiomixer_clone
                        .pad_template("sink_%u")
                        .expect("Failed to get audiomixer sink pad template");
                    let sinkpad = audiomixer_clone
                        .request_pad(&sinkpad_template, None, None)
                        .unwrap();
                    let srcpad = conv_clone.static_pad("src").unwrap();
                    srcpad
                        .link(&sinkpad)
                        .expect("Failed to link conv and audiomixer");
                }
                None
            });

            udpsrc
                .link(&rtpbin)
                .expect("Failed to link udpsrc and rtpbin");
            gstreamer::Element::link_many(&[&depay, &parse, &dec, &conv])?;
        }

        pipeline.add_many(&[&audiomixer, &opusenc, &rtpopuspay, &udpsink])?;
        gstreamer::Element::link_many(&[&audiomixer, &opusenc, &rtpopuspay, &udpsink])?;

        Ok(AudioMixerPipeline {
            pipeline: Arc::new(Mutex::new(Some(pipeline))),
            input_ports,
        })
    }

    pub async fn run(&self) -> Result<(), Box<dyn Error>> {
        println!("run");
        let pipeline = self.pipeline.lock().unwrap().as_ref().unwrap().clone();

        pipeline.set_state(gstreamer::State::Playing)?;

        let bus = pipeline.bus().unwrap();
        let msg = bus.timed_pop_filtered(gstreamer::ClockTime::NONE, &[]);
        if let Some(msg) = msg {
            match msg.view() {
                gstreamer::MessageView::Error(err) => {
                    let _ = pipeline.set_state(gstreamer::State::Null);
                    let debug = err.debug().unwrap_or_else(|| glib::GString::from("None"));
                    let error_msg = format!(
                        "Error from {:?}: {:?} ({:?})",
                        err.src()
                            .map(|s| s.path_string())
                            .unwrap_or_else(|| glib::GString::from("None")),
                        err.error(),
                        debug
                    );

                    element_error!(pipeline, gstreamer::LibraryError::Failed, (&error_msg));
                    Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        error_msg,
                    )))
                }
                gstreamer::MessageView::Eos(_) => {
                    let _ = pipeline.set_state(gstreamer::State::Null);
                    Ok(())
                }
                _ => {
                    let _ = pipeline.set_state(gstreamer::State::Null);
                    Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Unknown message received",
                    )))
                }
            }
        } else {
            let _ = pipeline.set_state(gstreamer::State::Null);
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No message received",
            )))
        }
    }

    pub fn get_input_ports(&self) -> Vec<u16> {
        self.input_ports.clone()
    }
}
