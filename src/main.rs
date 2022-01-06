 //use std::env;
use gstreamer::MessageView;
use gstreamer::element_error;
//use gstreamer::element_warning;
use gstreamer::prelude::*;

use anyhow::Error;
use derive_more::{Display, Error};

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

 // Connect source pad to rtpbin
fn connect_rtpbin_srcpad(src_pad: &gstreamer::Pad, sink: &gstreamer::Element) -> Result<(), Error> {
    println!("LOL CONNECTING PAD??");
    let name = src_pad.name();
    let split_name = name.split('_');
    let split_name = split_name.collect::<Vec<&str>>();
    let pt = split_name[5].parse::<u32>()?;

    match pt {
        100 => {
            println!("LOL 100 YAY!");
            let sinkpad = static_pad(sink, "sink")?;
            println!("SINK PAD IS: {:?}", sinkpad );
            println!("SRC PAD IS: {:?}", src_pad );
            src_pad.link(&sinkpad)?;
            Ok(())
        }
        _ => Err(Error::from(UnknownPT(pt))),
    }
}

#[doc(alias = "get_static_pad")]
fn static_pad(element: &gstreamer::Element, pad_name: &'static str) -> Result<gstreamer::Pad, Error> {
    match element.static_pad(pad_name) {
        Some(pad) => Ok(pad),
        None => {
            let element_name = element.name();
            Err(Error::from(NoSuchPad(pad_name, element_name.to_string())))
        }
    }
}


fn run_pipeline() -> Result<(), Error> {
    println!("Hello, world!");

    gstreamer::init()?;

    let  pipeline = gstreamer::Pipeline::new(Some("TestPipeline"));

    let src = gstreamer::ElementFactory::make("udpsrc", Some("UDP Src"))
                                         .map_err(|_| MissingElement("UDPSrc"))?;

    let queue = gstreamer::ElementFactory::make("queue", Some("Queue #1"))
                                           .map_err(|_| MissingElement("UDPSrc"))?;

    let rtpbin = gstreamer::ElementFactory::make("rtpbin", Some("RTPBin"))
                                            .map_err(|_| MissingElement("UDPSrc"))?;

    //let rtpopusdepay = gstreamer::ElementFactory::make("rtpopusdepay", Some("RTP Opus Depay"))
    //                                              .map_err(|_| MissingElement("UDPSrc"))?;

    //let opusparsein = gstreamer::ElementFactory::make("opusparse", Some("Opus Input Parser"))
    //                                             .map_err(|_| MissingElement("UDPSrc"))?;

    //let opusdec = gstreamer::ElementFactory::make("opusdec", Some("Opus Decode"))
    //                                         .map_err(|_| MissingElement("UDPSrc"))?;

    let audiomixer = gstreamer::ElementFactory::make("audiomixer", Some("Audio Mixer"))
                                                .map_err(|_| MissingElement("UDPSrc"))?;

    let opusenc = gstreamer::ElementFactory::make("opusenc", Some("Opus Encoder"))
                                             .map_err(|_| MissingElement("UDPSrc"))?;

    let opusparseout = gstreamer::ElementFactory::make("opusparse", Some("Opus Output Parser"))
                                                  .map_err(|_| MissingElement("UDPSrc"))?;

    let oggmux = gstreamer::ElementFactory::make("oggmux", Some("OGG Constructor"))
                                            .map_err(|_| MissingElement("UDPSrc"))?;

    let filesink = gstreamer::ElementFactory::make("filesink", Some("Write File as Ouput"))
                                              .map_err(|_| MissingElement("UDPSrc"))?;



       // Tell the filesrc what file to load
       src.set_property("port", 1925);
       src.set_property("caps",
           &gstreamer::Caps::builder("application/x-rtp")
                              .field("media", "audio")
                              .field("clock-rate", 48000)
                              .field("encoding-name", "OPUS")
                              .build()
       );
       
       filesink.set_property("location", "rust.ogg");
       

        pipeline.add_many(&[&src,
                            &queue,
                            &rtpbin,
                            //&rtpopusdepay,
                            //&opusparsein,
                            //&opusdec,
                            &audiomixer,
                            &opusenc,
                            &opusparseout,
                            &oggmux,
                            &filesink
                            ])?;

        //Link UDP source to rtpbin
        //let rtp_udp_src_pad = src.static_pad("src");
        //let rtp_recv_sink_pad = rtpbin.request_pad_simple("recv_rtp_sink_0");
        //rtp_udp_src_pad.link(&rtp_recv_sink_pad).expect("Failed to link udpsrc to rtpsink0");


        gstreamer::Element::link_many(&[&src,
                                        &queue,
                                        &rtpbin
                                        ])?;

        //gstreamer::Element::link_many(&[&audiomixer,
        //                                &opusenc,
        //                                &opusparseout,
        //                                &oggmux,
        //                                &filesink
        //                                ])?;


 //Set action to take when payload type is known to rtpbin
 rtpbin.connect("request-pt-map", false, |values| {
        let pt = values[2]
            .get::<u32>()
            .expect("rtpbin \"new-storage\" signal values[2]");
        println!("RTPBin got payload of type {:?}", pt );
        match pt {
            100 => Some(
                gstreamer::Caps::builder("application/x-rtp")
                    .field("media", "audio")
                    .field("clock-rate", 48000i32)
                    .field("encoding-name", "OPUS")
                    .build()
                    .to_value(),
            ),
            96 => Some(
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


 //Set action to take when pad is added to rtpbin
 // (connect this pad to a depayloader, parser, decoder, and then into the mixer)
        let pipeline_weak = pipeline.downgrade();
 rtpbin.connect_pad_added(
   move |rtpbin, src_pad| {
        println!("New source pad added to RTPBin");
        println!("Creating new elements to handle new RTP stream");

        let pipeline_strong = match pipeline_weak.upgrade() {
                     Some(pipeline) => pipeline,
                     None => return
         };
  
        //Make rtpopusdepay, opus parsein, opusdec elements
        let rtpopusdepay = gstreamer::ElementFactory::make("rtpopusdepay", Some("RTP Opus Depay"))
                           .expect("Can not make RTP opus depayloader for new RTP media");
        let opusparsein = gstreamer::ElementFactory::make("opusparse", Some("Opus Input Parser"))
                          .expect("Can not make opus parser for new RTP media");
        let opusdec = gstreamer::ElementFactory::make("opusdec", Some("Opus Decode"))
                      .expect("Can not make opus decoder for new RTP media");

        pipeline_strong.add_many(&[&rtpopusdepay,
                                   &opusparsein,
                                   &opusdec])
                .expect("Can not add elements to pipeline!");

        gstreamer::Element::link_many(&[&rtpopusdepay,
                                        &opusparsein,
                                        &opusdec,
                                        &audiomixer,
                                        &opusenc,
                                        &opusparseout,
                                        &oggmux,
                                        &filesink
                                        ]);


        //Connect sourcepad to depayloader
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

        rtpopusdepay.sync_state_with_parent();
        opusparsein.sync_state_with_parent();
        opusdec.sync_state_with_parent();
        audiomixer.sync_state_with_parent();
        opusenc.sync_state_with_parent();
        opusparseout.sync_state_with_parent();
        oggmux.sync_state_with_parent();
        filesink.sync_state_with_parent();
    });



        //Play Gstreamer pipeline
        pipeline.set_state(gstreamer::State::Playing);

        //Expect pipeline has bus
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
                    debug: err.debug(),
                    source: err.error(),
                }
                .into());
            }
            MessageView::StateChanged(s) => {
                if let Some(element) = msg.src() {
                    if element == pipeline && s.current() == gstreamer::State::Playing {
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
              println!("Warning: {:?}", msg.src() )
            }
            MessageView::Info(s) => {
              println!("Warning: {:?}", msg.src() )
            }
            MessageView::Tag(s) => {
              println!("Tag: {:?}", msg.src() )
            }
            MessageView::StreamStatus(s) => {
              println!("Stream Status: {:?} and then {:?}", msg, s )
            }



            _ => {println!("Unknown {:?}", msg )},
        }
    }

    pipeline
        .set_state(gstreamer::State::Null)
        .expect("Unable to set the pipeline to the `Null` state");

    Ok(())
}

fn main() {
  match run_pipeline() {
      Ok(r) => r,
      Err(e) => eprintln!("Error! {}", e)
  }
  println!("Gstreamer Is gone");
}
