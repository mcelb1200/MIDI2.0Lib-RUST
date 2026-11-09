//! Platform MIDI I/O backend using `midir`.

use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use std::error::Error;

/// Port description containing index and friendly name.
#[derive(Clone, Debug)]
pub struct PortInfo {
    pub index: usize,
    pub name: String,
}

/// Enumerates all available MIDI input ports on the host platform.
pub fn list_input_ports() -> Result<Vec<PortInfo>, Box<dyn Error>> {
    let midi_in = MidiInput::new("el_bridge_probe")?;
    let mut ports = Vec::new();
    for (i, p) in midi_in.ports().iter().enumerate() {
        let name = midi_in.port_name(p).unwrap_or_else(|_| "Unknown".into());
        ports.push(PortInfo { index: i, name });
    }
    Ok(ports)
}

/// Enumerates all available MIDI output ports on the host platform.
pub fn list_output_ports() -> Result<Vec<PortInfo>, Box<dyn Error>> {
    let midi_out = MidiOutput::new("el_bridge_probe")?;
    let mut ports = Vec::new();
    for (i, p) in midi_out.ports().iter().enumerate() {
        let name = midi_out.port_name(p).unwrap_or_else(|_| "Unknown".into());
        ports.push(PortInfo { index: i, name });
    }
    Ok(ports)
}

/// Connects to a MIDI input port by index, passing incoming bytes directly to the callback.
pub fn connect_input<F>(
    port_index: usize,
    callback: F,
) -> Result<MidiInputConnection<()>, Box<dyn Error>>
where
    F: FnMut(u64, &[u8], &mut ()) + Send + 'static,
{
    let midi_in = MidiInput::new("el_bridge_in")?;
    let ports = midi_in.ports();
    let port = ports
        .get(port_index)
        .ok_or_else(|| format!("Input port index {} not found", port_index))?;
    let conn = midi_in.connect(port, "el_bridge_rx", callback, ())?;
    Ok(conn)
}

/// Connects to a MIDI output port by index.
pub fn connect_output(port_index: usize) -> Result<MidiOutputConnection, Box<dyn Error>> {
    let midi_out = MidiOutput::new("el_bridge_out")?;
    let ports = midi_out.ports();
    let port = ports
        .get(port_index)
        .ok_or_else(|| format!("Output port index {} not found", port_index))?;
    let conn = midi_out.connect(port, "el_bridge_tx")?;
    Ok(conn)
}
