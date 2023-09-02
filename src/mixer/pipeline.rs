use anyhow::Error;
use derive_more::{Display, Error};
use glib::translate::FromGlib;
use gstreamer::traits::ElementExt;
use gstreamer::{element_error, prelude::*, Element, MessageView};
use gstreamer::{ElementFactory, Pipeline};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

//Helper functions
#[derive(Debug, Display, Error)]
#[display(fmt = "Missing element {}", _0)]
struct MissingElement(#[error(not(source))] &'static str);

#[derive(Debug, Display, Error)]
#[display(fmt = "Received error from {}: {} (debug: {:?})", src, error, debug)]
struct ErrorMessage {
    src: String,
    error: String,
    debug: Option<String>,
    source: glib::Error,
}

#[cfg(feature = "v1_10")]
#[derive(Clone, Debug, glib::Boxed)]
#[boxed_type(name = "ErrorValue")]
struct ErrorValue(Arc<Mutex<Option<Error>>>);

#[derive(Debug, Display, Error)]
#[display(fmt = "Unknown payload type {}", _0)]
struct UnknownPT(#[error(not(source))] u32);

#[derive(Debug, Display, Error)]
#[display(fmt = "No such pad {} in {}", _0, _1)]
struct NoSuchPad(#[error(not(source))] &'static str, String);

pub struct AudioMixerPipeline {
    pipeline: Arc<Mutex<Option<Pipeline>>>,
    input_ports: Vec<u16>,
}

impl AudioMixerPipeline {
    pub fn new(
        input_ports: Vec<u16>,
        destination_ip: &str,
        destination_port: u16,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        gstreamer::init()?;
        let pipeline = Pipeline::new(Some("FrameMixerPipeline"));
        let src = ElementFactory::make("udpsrc")
            .build()
            .expect("failed to create udpsrc");
        let rtpbin = ElementFactory::make("rtpbin")
            .build()
            .expect("failed to create rtpbin");

        // create mixer
        let audiomixer = ElementFactory::make("audiomixer")
            .build()
            .expect("failed to create audiomixer");
        let opusenc = ElementFactory::make("opusenc")
            .build()
            .expect("failed to create OpusEnc");
        let opusparseout = ElementFactory::make("opusparse")
            .build()
            .expect("failed to create OpusParseOut");
        let rtpopuspay = ElementFactory::make("rtpopuspay")
            .build()
            .expect("failed to create RTPOpusPay");
        let udpsink = ElementFactory::make("udpsink")
            .build()
            .expect("failed to create UDPSink");
        let audio_caps = gstreamer::Caps::builder("application/x-rtp")
            .field("media", "audio")
            .field("clock-rate", 48000)
            .field("encoding-name", "OPUS")
            .build();
        src.set_property("port", 1925); //TODO: Get this from signaling
        src.set_property("caps", &audio_caps);

        opusenc.set_property("bitrate", 48000);

        udpsink.set_property("host", "127.0.0.1"); //TODO: Get this from signaling
                                                   //udpsink.set_property("host", "127.0.0.1")?; //TODO: Get this from signaling
        udpsink.set_property("port", 1928); //TODO: Get this from signaling
                                            // Add elements to the pipeline
        pipeline.add_many(&[
            &src,
            &rtpbin,
            &audiomixer,
            &opusenc,
            &opusparseout,
            &rtpopuspay,
            &udpsink, //&oggmux,
                      //&filesink
        ])?;

        gstreamer::Element::link_many(&[&src, &rtpbin])?;

        // Respond to determining payload type (audio, video)
        rtpbin.connect("request-pt-map", false, |values| {
            let pt = values[2]
                .get::<u32>()
                .expect("rtpbin \"new-storage\" signal values[2]");
            println!("RTPBin got payload of type {:?}", pt);
            match pt {
                100 => Some(
                    gstreamer::Caps::builder("application/x-rtp")
                        .field("media", "audio")
                        .field("clock-rate", 48000i32)
                        .field("encoding-name", "OPUS")
                        .build()
                        .to_value(),
                ),
                106 => Some(
                    gstreamer::Caps::builder("application/x-rtp")
                        .field("media", "video")
                        .field("clock-rate", 90000i32)
                        .field("encoding-name", "VP8")
                        .build()
                        .to_value(),
                ),
                _ => None,
            }
        });

        //This is the outgoing SSRC to egress. Send this value to egres
        // rtpbin.connect("on-new-sender-ssrc", false, |values| {
        //     println!("@@ON NEW SENDER SSRC!!! {:?}", values);
        //     if let [_, _, ssrc_value] = values {
        //         if let Ok(ssrc) = ssrc_value.get::<u32>() {
        //             println!("SSRC Value: {}", ssrc);
        //         } else {
        //             println!("Failed to extract SSRC value.");
        //         }
        //     }
        //     None
        // });

        //This is incoming from ingress... dont use?
        // rtpbin.connect("on-ssrc-validated", false, |values| {
        //     println!("@@ON SSRC VALIDATED!!! {:?}", values);
        //     if let [_, _, ssrc_value] = values {
        //         if let Ok(ssrc) = ssrc_value.get::<u32>() {
        //             println!("on new SSRC Value: {}", ssrc);
        //         } else {
        //             println!("Failed to extract SSRC value.");
        //         }
        //     }
        //     None
        // });

        //This is a new ssrc from ingress. Dont send this one
        rtpbin.connect("on-new-ssrc", false, |values| {
            println!("ON NEW SSRC!!! {:?}", values);
            if let [_, _, ssrc_value] = values {
                if let Ok(ssrc) = ssrc_value.get::<u32>() {
                    println!("on new SSRC Value: {}", ssrc);
                } else {
                    println!("Failed to extract SSRC value.");
                }
            }
            None
        });
        rtpbin.connect("on-ssrc-sdes", false, |values| {
            println!("ON SSRC SDES!!! {:?}", values);
            None
        });

        // Some payload type changed?
        rtpbin.connect("payload-type-change", false, |values| {
            println!("ON PAYLOAD CHANGE!!!! {:?}", values);
            None
        });

        //Set action to take when pad is added to rtpbin
        // (connect this pad to a depayloader, parser, decoder, and then into the mixer)
        let pipeline_weak = pipeline.downgrade(); //Downgrade to use in function
        rtpbin.connect_pad_added(move |rtpbin, src_pad| {
            println!("New source pad added to RTPBin");
            println!("Creating new elements to handle new RTP stream");

            let pipeline_strong = match pipeline_weak.upgrade() {
                Some(pipeline) => pipeline,
                None => return,
            }; //Upgrade to use in function

            //Make elements that will handle this new incoming stream
            let rtpopusdepay = gstreamer::ElementFactory::make("rtpopusdepay")
                .build()
                .expect("Can not make RTP opus depayloader for new RTP media");
            let opusparsein = gstreamer::ElementFactory::make("opusparse")
                .build()
                .expect("Can not make opus parser for new RTP media");
            let opusdec = gstreamer::ElementFactory::make("opusdec")
                .build()
                .expect("Can not make opus decoder for new RTP media");

            //Add elements to the pipeline
            pipeline_strong
                .add_many(&[&rtpopusdepay, &opusparsein, &opusdec])
                .expect("Can not add elements to pipeline!");

            //Link the elements from the depayload to the output
            let _ = gstreamer::Element::link_many(&[
                &rtpopusdepay,
                &opusparsein,
                &opusdec,
                &audiomixer,
                &opusenc,
                &opusparseout,
                &rtpopuspay,
                &udpsink, //&oggmux,
                          //&filesink
            ]);
            //rtpopuspay.set_property("pt", 100);

            //Connect new rtpbin srcpad to the linked elements
            // (this completes the pipe from the new media to the end output)
            match connect_rtpbin_srcpad(src_pad, &rtpopusdepay) {
                Ok(_) => (),
                Err(err) => {
                    element_error!(
                        rtpbin,
                        gstreamer::LibraryError::Failed,
                        ("Failed to link srcpad"),
                        ["{}", err]
                    );
                }
            }

            //This is important for elements not getting confused about time
            rtpopusdepay
                .sync_state_with_parent()
                .expect("Can not sync element state with parent!");
            opusparsein
                .sync_state_with_parent()
                .expect("Can not sync element state with parent!");
            opusdec
                .sync_state_with_parent()
                .expect("Can not sync element state with parent!");
            audiomixer
                .sync_state_with_parent()
                .expect("Can not sync element state with parent!");
        });

        Ok(Self {
            pipeline: Arc::new(Mutex::new(Some(pipeline))),
            input_ports,
        })
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("run");
        let pipeline = self.pipeline.lock().unwrap().as_ref().unwrap().clone();

        pipeline.set_state(gstreamer::State::Playing)?;
        let bus = pipeline
            .bus()
            .expect("Pipeline without bus. Shouldn't happen!");

        //Loop and move pipeline forward
        for msg in bus.iter_timed(gstreamer::ClockTime::NONE) {
            match msg.view() {
                MessageView::Eos(..) => break,
                MessageView::Error(err) => {
                    pipeline
                        .set_state(gstreamer::State::Null)
                        .expect("Unable to set the pipeline to the `Null` state");

                    return Err(ErrorMessage {
                        src: msg
                            .src()
                            .map(|s| String::from(s.path_string()))
                            .unwrap_or_else(|| String::from("None")),
                        error: err.error().to_string(),
                        debug: err.debug().map(|gstr| gstr.as_str().to_owned()),
                        source: err.error(),
                    }
                    .into());
                }
                MessageView::StreamStart(s) => {
                    println!("Received a StreamStart message: {:?}", s);
                    // Additional handling of StreamStart messages can go here.
                }
                MessageView::Latency(s) => {
                    println!("Received a Latency message: {:?}", s);
                    // Additional handling of Latency messages can go here.
                }
                MessageView::AsyncDone(s) => {
                    println!("Received an AsyncDone message: {:?}", s);
                    // Additional handling of AsyncDone messages can go here.
                }
                MessageView::Element(el) => {
                    println!("Received an Element message: {:?}", el);
                    // Additional handling of element messages can go here.
                }
                MessageView::StateChanged(s) => {
                    if let Some(element) = msg.src() {
                        if element.clone() == pipeline && s.current() == gstreamer::State::Playing {
                            eprintln!("PLAYING");
                            gstreamer::debug_bin_to_dot_file(
                                &pipeline,
                                gstreamer::DebugGraphDetails::all(),
                                "client-playing",
                            );
                        }
                    }
                }
                MessageView::Warning(s) => {
                    println!("Warning: {:?} {:?}", s, msg.src())
                }
                MessageView::Info(s) => {
                    println!("Warning: {:?} {:?}", s, msg.src())
                }
                MessageView::Tag(s) => {
                    println!("Tag: {:?} {:?}", s, msg.src())
                }
                MessageView::StreamStatus(s) => {
                    println!("Stream Status: {:?} and then {:?}", msg, s)
                }

                _ => {
                    println!("Unknown {:?}", msg)
                }
            }
        }

        //Stop playing pipeline
        pipeline
            .set_state(gstreamer::State::Null)
            .expect("Unable to set the pipeline to the `Null` state");

        Ok(())
    }

    pub fn get_input_ports(&self) -> Vec<u16> {
        self.input_ports.clone()
    }
}

// Connect source pad to rtpbin
fn connect_rtpbin_srcpad(src_pad: &gstreamer::Pad, sink: &gstreamer::Element) -> Result<(), Error> {
    let name = src_pad.name();
    let split_name = name.split('_');
    let split_name = split_name.collect::<Vec<&str>>();
    let pt = split_name[5].parse::<u32>()?;

    match pt {
        100 => {
            println!("Payload type is 100 YAY!");
            let sinkpad = static_pad(sink, "sink");
            let _ = src_pad.link(&sinkpad.unwrap());
            Ok(())
        }
        _ => Err(Error::from(UnknownPT(pt))),
    }
}

#[doc(alias = "get_static_pad")]
fn static_pad(
    element: &gstreamer::Element,
    pad_name: &'static str,
) -> Result<gstreamer::Pad, Error> {
    match element.static_pad(pad_name) {
        Some(pad) => Ok(pad),
        None => {
            let element_name = element.name();
            Err(Error::from(NoSuchPad(pad_name, element_name.to_string())))
        }
    }
}
