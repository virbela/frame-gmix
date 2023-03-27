use uuid::serde;

use super::port_range_manager::PortRangeManager;
use crate::mixer::pipeline::AudioMixerPipeline;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};

pub struct MixerSessionManager {
    sessions: Arc<Mutex<HashMap<String, AudioMixerPipeline>>>,
    port_range_manager: Mutex<PortRangeManager>,
}

impl MixerSessionManager {
    pub fn new(port_range: (u16, u16)) -> Self {
        MixerSessionManager {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            port_range_manager: Mutex::new(PortRangeManager::new(port_range.0, port_range.1)),
        }
    }

    pub fn create_session(
        &self,
        session_id: String,
        num_input_ports: usize,
        destination_ip: &str,
        destination_port: u16,
    ) -> Result<(), Box<dyn Error>> {
        let input_ports = {
            let mut port_manager = self.port_range_manager.lock().unwrap();
            port_manager.allocate_ports(num_input_ports)?
        };
        println!("after inputports");
        println!(
            "inpurt_ports: {:?}, destination_ip: {:?}, deallocate_ports: {:?}",
            input_ports.clone(),
            destination_ip.clone(),
            destination_port.clone()
        );
        let audio_mixer_pipeline =
            AudioMixerPipeline::new(input_ports.clone(), destination_ip, destination_port)?;
        println!("after audio_mixer_pipeline");
        self.sessions
            .lock()
            .unwrap()
            .insert(session_id.clone(), audio_mixer_pipeline);
        println!("before ok");
        Ok(())
    }

    pub async fn start_session(&self, session_id: &str) -> Result<(), Box<dyn Error>> {
        let sessions = self.sessions.lock().unwrap();
        if let Some(audio_mixer_pipeline) = sessions.get(session_id) {
            audio_mixer_pipeline.run().await?;
            Ok(())
        } else {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Session {} not found", session_id),
            )))
        }
    }

    pub fn remove_session(&self, session_id: &str) -> Result<(), Box<dyn Error>> {
        let removed_session = self.sessions.lock().unwrap().remove(session_id);

        if let Some(audio_mixer_pipeline) = removed_session {
            let input_ports = audio_mixer_pipeline.get_input_ports();
            let mut port_manager = self.port_range_manager.lock().unwrap();
            port_manager.deallocate_ports(&input_ports);
            Ok(())
        } else {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Session {} not found", session_id),
            )))
        }
    }
}
