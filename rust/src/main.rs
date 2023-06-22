use gstreamer::prelude::*;
use anyhow::Error;
use derive_more::{Display, Error};

//Helper functions
#[derive(Debug, Display, Error)]
#[display(fmt = "Received error from {src}: {error} (debug: {debug:?})")]
struct ErrorMessage {
    src: glib::GString,
    error: glib::Error,
    debug: Option<glib::GString>,
}
#[derive(Debug, Display, Error)]
#[display(fmt = "Unknown payload type {}", _0)]
struct UnknownPT(#[error(not(source))] u32);

// Initialize gstreamer pipeline
//  and set functions for when receiving a new mediasource
fn create_pipeline() -> Result<gstreamer::Pipeline, Error> {

    gstreamer::init()?;

    let pipeline = gstreamer::Pipeline::default();

    //Construct elements
    let udpsrc = gstreamer::ElementFactory::make("udpsrc").build()?;
    let rtpbin = gstreamer::ElementFactory::make("rtpbin").build()?;
    let audiomixer = gstreamer::ElementFactory::make("audiomixer").build()?;
    let opusenc = gstreamer::ElementFactory::make("opusenc").build()?;
    let opusparseout = gstreamer::ElementFactory::make("opusparse").build()?;
    let rtpopuspay = gstreamer::ElementFactory::make("rtpopuspay").build()?;
    let udpsink = gstreamer::ElementFactory::make("udpsink").build()?;

    // Configure elements
    let audio_caps =  gstreamer::Caps::builder("application/x-rtp")
                                      .field("media", "audio")
                                      .field("clock-rate", 48000)
                                      .field("encoding-name", "OPUS")
                                      .build();
    udpsrc.set_property("port", 1925); //TODO: Get this from signaling
    udpsrc.set_property("caps", &audio_caps);
    
    opusenc.set_property("bitrate", 48000);
    
    udpsink.set_property("host", "127.0.0.1"); //TODO: Get this from signaling
    udpsink.set_property("port", 1928); //TODO: Get this from signaling
    
    // Add elements to the pipeline
    pipeline.add_many(&[&udpsrc,
                        &rtpbin,
                        &audiomixer,
                        &opusenc,
                        &opusparseout,
                        &rtpopuspay,
                        &udpsink
                        ])?;

    // Link the elements to other elements
    // Each udpsrc should connect to rtpbin
    gstreamer::Element::link_many(&[&udpsrc,
                                    &rtpbin
                                    ])?;


    //Set rtpbin handlers
    // Respond to determining payload type (audio, video)
    rtpbin.connect("request-pt-map", false, |values| {
        let pt = values[2]
                 .get::<u32>()
                 .expect("rtpbin new-storage signal values[2]");
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

    // Respond to new pad added to rtpbin
    // (connect this pad to a depayloader, parser, decoder, and then into the mixer)
    let pipeline_weak = pipeline.downgrade(); //Downgrade to use in function
    rtpbin.connect_pad_added( move |_rtpbin, src_pad| {
        println!("New source pad added to RTPBin");
        println!("Creating new elements to handle new RTP stream");
    
       let pipeline_strong = match pipeline_weak.upgrade() {
                     Some(pipeline) => pipeline,
                     None => return
         }; //Upgrade to use in function
    
        //Make elements that will handle this new incoming stream
        let rtpopusdepay = gstreamer::ElementFactory::make("rtpopusdepay").build().expect("sht1");
        let opusparsein = gstreamer::ElementFactory::make("opusparse").build().expect("sht2");
        let opusdec = gstreamer::ElementFactory::make("opusdec").build().expect("sht3");
    
        //Add elements to the pipeline
        pipeline_strong.add_many(&[&rtpopusdepay,
                                   &opusparsein,
                                   &opusdec]).expect("Can not add to ppelne!");

        //Link the elements from the depayload to the output
        gstreamer::Element::link_many(&[&rtpopusdepay,
                                        &opusparsein,
                                        &opusdec,
                                        &audiomixer,
                                        &opusenc,
                                        &opusparseout,
                                        &rtpopuspay,
                                        &udpsink
                                       ]).expect("Can not link new elements to pipeline!");


        //Connect new rtpbin srcpad to the linked elements
        // (this completes the pipe from the new media to the end output)
        println!("LOL CONNECTING PAD??");
        let name = src_pad.name();
        let split_name = name.split('_');
        let split_name = split_name.collect::<Vec<&str>>();
        let pt = split_name[5].parse::<u32>().expect("Can't parse src pad name!");

        match pt {
            96 => {
                println!("Lnk 96 YAY!");
                let sinkpad = rtpopusdepay.static_pad("sink").expect("Can't get static pad!");
                src_pad.link(&sinkpad).expect("Can't link src_pad!");
            }
            100 => {
                println!("Lnk 100 YAY!");
                let sinkpad = rtpopusdepay.static_pad("sink").expect("Can't get static pad!");
                src_pad.link(&sinkpad).expect("Can't link src_pad!");
            }
            _ => Err::<(), Error>(Error::from(UnknownPT(pt))).unwrap(),
        };
    
    });

    Ok(pipeline)
}

//Gstreamer loop
fn loop_pipeline(pipeline: gstreamer::Pipeline) -> Result<(), Error> {
    pipeline.set_state(gstreamer::State::Playing)?;

    let bus = pipeline
        .bus()
        .expect("Pipeline without bus. Shouldn't happen!");

    for msg in bus.iter_timed(gstreamer::ClockTime::NONE) {
        use gstreamer::MessageView;

        match msg.view() {
            //MessageView::Eos(..) => break,
            MessageView::Error(err) => {
                pipeline.set_state(gstreamer::State::Null)?;
                return Err(ErrorMessage {
                    src: msg
                        .src()
                        .map(|s| s.path_string())
                        .unwrap_or_else(|| glib::GString::from("UNKNOWN")),
                    error: err.error(),
                    debug: err.debug(),
                }
                .into());
            }
            MessageView::StateChanged(state) => {
                println!("{:?}", state);
            }
            MessageView::StreamStatus(status) => {
                println!("Received new status change: {:?}", status);
            }
            MessageView::NewClock(clock) => {
                println!("Received new clock change: {:?}", clock);
            }
            _ => {
                println!("Received message of type: {:?}", msg.type_());
            }

        }
    }
    pipeline.set_state(gstreamer::State::Null)?;

    Ok(())
}



fn main() {
  match create_pipeline().and_then(loop_pipeline) {
      Ok(r) => r,
      Err(e) => eprintln!("Error! {}", e)
  }
  println!("Gstreamer process complete.");
}
