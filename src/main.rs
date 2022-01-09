use gstreamer::MessageView;
use gstreamer::element_error;
use gstreamer::prelude::*;
use anyhow::Error;
use derive_more::{Display, Error};

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

// Initialize gstreamer pipeline
//  and set functions for when receiving a new mediasource
fn run_pipeline() -> Result<(), Error> {

    gstreamer::init()?;

    let pipeline = gstreamer::Pipeline::new(Some("TestPipeline"));

    //Create 7 inputs to the mediapipe
    //TODO: Find out how to make one port handle each room
    let src1 = gstreamer::ElementFactory::make("udpsrc", Some("UDP Src 1925"))
                                         .map_err(|_| MissingElement("UDPSrc"))?;
    //let src2 = gstreamer::ElementFactory::make("udpsrc", Some("UDP Src 1926"))
    //                                     .map_err(|_| MissingElement("UDPSrc"))?;
    //let src3 = gstreamer::ElementFactory::make("udpsrc", Some("UDP Src 1927"))
    //                                     .map_err(|_| MissingElement("UDPSrc"))?;
    //let src4 = gstreamer::ElementFactory::make("udpsrc", Some("UDP Src 1928"))
    //                                     .map_err(|_| MissingElement("UDPSrc"))?;
    //let src5 = gstreamer::ElementFactory::make("udpsrc", Some("UDP Src 1929"))
    //                                     .map_err(|_| MissingElement("UDPSrc"))?;
    //let src6 = gstreamer::ElementFactory::make("udpsrc", Some("UDP Src 1930"))
    //                                     .map_err(|_| MissingElement("UDPSrc"))?;
    //let src7 = gstreamer::ElementFactory::make("udpsrc", Some("UDP Src 1931"))
    //                                     .map_err(|_| MissingElement("UDPSrc"))?;

    //Create rtpbin that can accept multiple rtp sessions
    let rtpbin = gstreamer::ElementFactory::make("rtpbin", Some("RTPBin"))
                                            .map_err(|_| MissingElement("UDPSrc"))?;

    //Create audio mixer and output to file
    //TODO output to mediasoup
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


    // Configure elements
    let audioCaps =  gstreamer::Caps::builder("application/x-rtp")
                                      .field("media", "audio")
                                      .field("clock-rate", 48000)
                                      .field("encoding-name", "OPUS")
                                      .build();
    src1.set_property("port", 1925); //TODO: Get this from signaling
    //src2.set_property("port", 1926);
    //src3.set_property("port", 1927);
    //src4.set_property("port", 1928);
    //src5.set_property("port", 1929);
    //src6.set_property("port", 1930);
    //src7.set_property("port", 1931);
    src1.set_property("caps", &audioCaps);
    //src2.set_property("caps", &audioCaps);
    //src3.set_property("caps", &audioCaps);
    //src4.set_property("caps", &audioCaps);
    //src5.set_property("caps", &audioCaps);
    //src6.set_property("caps", &audioCaps);
    //src7.set_property("caps", &audioCaps);
    
    //TODO: Hook this into mediasoup
    filesink.set_property("location", "rust.ogg");
       

    // Add elements to the pipeline
    pipeline.add_many(&[&src1,
                        //&src2,
                        //&src3,
                        //&src4,
                        //&src5,
                        //&src6,
                        //&src7,
                        &rtpbin,
                        &audiomixer,
                        &opusenc,
                        &opusparseout,
                        &oggmux,
                        &filesink
                        ])?;

    // Link the elements to other elements
    // Each udpsrc should connect to rtpbin
    gstreamer::Element::link_many(&[&src1,
                                    &rtpbin
                                    ])?;
    //gstreamer::Element::link_many(&[&src2,
    //                                &rtpbin
    //                                ])?;
    //gstreamer::Element::link_many(&[&src3,
    //                                &rtpbin
    //                                ])?;
    //gstreamer::Element::link_many(&[&src4,
    //                                &rtpbin
    //                                ])?;
    //gstreamer::Element::link_many(&[&src5,
    //                                &rtpbin
    //                                ])?;
    //gstreamer::Element::link_many(&[&src6,
    //                                &rtpbin
    //                                ])?;
    //gstreamer::Element::link_many(&[&src7,
    //                                &rtpbin
    //                                ])?;

    // Respond to determining payload type (audio, video)
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


    rtpbin.connect( "on-new-ssrc", false, |values| { 
      println!("NEW SRC!!");
        //let session = values[1];
        //let ssrc = values[2];
        //match values[1] {
        //    _ => None,
        //}
        None
    });
    //rtpbin.connect( "on-new-sender-ssrc", false, |values| { 
    //  println!("NEW SENDER SRC!!");
    //});
    //rtpbin.connect( "on-ssrc-sdes", false, |values| { 
    //  println!("NEW SESSION DATA!!!");
    //});



    //Set action to take when pad is added to rtpbin
    // (connect this pad to a depayloader, parser, decoder, and then into the mixer)
    let pipeline_weak = pipeline.downgrade(); //Downgrade to use in function
    rtpbin.connect_pad_added( move |rtpbin, src_pad| {
        println!("New source pad added to RTPBin");
        println!("Creating new elements to handle new RTP stream");

        let pipeline_strong = match pipeline_weak.upgrade() {
                     Some(pipeline) => pipeline,
                     None => return
         }; //Upgrade to use in function
  
        //Make elements that will handle this new incoming stream
        let rtpopusdepay = gstreamer::ElementFactory::make("rtpopusdepay", None)
                           .expect("Can not make RTP opus depayloader for new RTP media");
        let opusparsein = gstreamer::ElementFactory::make("opusparse", None)
                          .expect("Can not make opus parser for new RTP media");
        let opusdec = gstreamer::ElementFactory::make("opusdec", None)
                      .expect("Can not make opus decoder for new RTP media");

        //Add elements to the pipeline
        pipeline_strong.add_many(&[&rtpopusdepay,
                                   &opusparsein,
                                   &opusdec])
                                   .expect("Can not add elements to pipeline!");

        //Link the elements from the depayload to the output
        gstreamer::Element::link_many(&[&rtpopusdepay,
                                        &opusparsein,
                                        &opusdec,
                                        &audiomixer,
                                        &opusenc,
                                        &opusparseout,
                                        &oggmux,
                                        &filesink
                                        ]);


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

    //Stop playing pipeline
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
  println!("Gstreamer process complete.");
}
