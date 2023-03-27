use gstreamer::{element_error, prelude::*};

use gstreamer::traits::ElementExt;
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
            let queue = ElementFactory::make("queue")
                .build()
                .expect("failed to create queue element");
            let elements = (udpsrc, rtpbin, depay, queue.clone(), parse, dec, conv);
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
            .expect("failed to create rtpopusdepay");
        let udpsink = ElementFactory::make("udpsink").build().expect(
            "failed to create udpsink
        ",
        );

        udpsink.set_property_from_str("host", destination_ip);
        udpsink.set_property_from_str("port", &destination_port.to_string());

        let pipeline = Pipeline::new(None);
        println!("after pipeline");
        pipeline.add_many(&[&audiomixer, &opusenc, &rtpopuspay, &udpsink])?;
        println!("after pipeline add many");
        for (i, (udpsrc, rtpbin, depay, queue, parse, dec, conv)) in
            input_elements.into_iter().enumerate()
        {
            pipeline.add_many(&[&udpsrc, &rtpbin, &depay, &parse, &dec, &conv])?;
            println!("after pipeline add many 2");
            let link_result = gstreamer::Element::link_many(&[
                &udpsrc,
                &rtpbin,
                &depay,
                &queue,
                &parse,
                &dec,
                &conv,
                &audiomixer,
            ]);
            println!("after let link many result");
            if let Err(err) = link_result {
                let message = format!("Failed to link elements: {}", err);
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    message,
                )));
            }
            println!("after let error");
            let sinkpad_template = audiomixer
                .pad_template("sink_%u")
                .expect("Failed to get audiomixer sink pad template");
            println!("before panic ");
            let sinkpad = audiomixer
                .request_pad(&sinkpad_template, None, None)
                .unwrap();
            println!("after panic");
            let srcpad = conv.static_pad("src").unwrap();
            let link_result = srcpad.link(&sinkpad);
            if let Err(e) = link_result {
                eprintln!("Failed to link pads: {}", e);
                return Err(Box::new(e));
            }
        }

        gstreamer::Element::link_many(&[&audiomixer, &opusenc, &rtpopuspay, &udpsink])?;

        Ok(AudioMixerPipeline {
            pipeline: Arc::new(Mutex::new(Some(pipeline))),
            input_ports,
        })
    }

    pub async fn run(&self) -> Result<(), Box<dyn Error>> {
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
