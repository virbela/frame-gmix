use std::collections::HashSet;
use std::error::Error;

pub struct PortRangeManager {
    available_ports: Vec<u16>,
    used_ports: HashSet<u16>,
}

impl PortRangeManager {
    pub fn new(start: u16, end: u16) -> Self {
        PortRangeManager {
            available_ports: (start..=end).collect(),
            used_ports: HashSet::new(),
        }
    }

    pub fn allocate_ports(&mut self, count: usize) -> Result<Vec<u16>, Box<dyn Error>> {
        if self.available_ports.len() < count {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Not enough available ports",
            )));
        }

        let allocated_ports: Vec<u16> = self.available_ports.drain(..count).collect();
        for port in &allocated_ports {
            self.used_ports.insert(*port);
        }

        Ok(allocated_ports)
    }

    pub fn deallocate_ports(&mut self, ports: &[u16]) {
        for port in ports {
            if self.used_ports.remove(port) {
                self.available_ports.push(*port);
            }
        }
    }
}
