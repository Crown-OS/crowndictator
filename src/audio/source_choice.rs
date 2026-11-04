//! Which PipeWire node a recording captures.

use crownui_audio::{SourceId, SourceInfo};

/// The node to capture for the `dictation_microphone` named in `input.ron`,
/// or `None` for the system default.
///
/// A configured name that matches nothing falls back to the default rather than
/// failing. The device is a USB microphone that is currently unplugged far more
/// often than it is a typo, and refusing to record at all would be a strange
/// way to react to a headset being in the other room — the settings panel
/// already says so on the Input page, and the name stays in the file so it is
/// used again the moment the device is back.
///
/// An empty `connected` list means the graph has not been read yet, so the name
/// is passed on as is and PipeWire's own fallback to the default applies.
pub fn capture_source(wanted: Option<&str>, connected: &[SourceInfo]) -> Option<SourceId> {
    let wanted = wanted?;
    if connected.is_empty() {
        return Some(SourceId::new(wanted));
    }
    let found = connected
        .iter()
        .find(|source| source.id.as_str() == wanted)
        .map(|source| source.id.clone());
    if found.is_none() {
        log::warn!("audio: '{wanted}' is not connected; using the default input");
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADSET: &str = "alsa_input.usb-headset.mono-fallback";

    fn source(name: &str) -> SourceInfo {
        SourceInfo {
            id: SourceId::new(name),
            description: name.to_owned(),
            is_default: false,
        }
    }

    #[test]
    fn no_name_means_the_default() {
        assert_eq!(capture_source(None, &[source(HEADSET)]), None);
    }

    #[test]
    fn a_connected_name_is_captured() {
        let connected = [source("alsa_input.pci.analog-stereo"), source(HEADSET)];
        assert_eq!(
            capture_source(Some(HEADSET), &connected),
            Some(SourceId::new(HEADSET))
        );
    }

    #[test]
    fn an_unplugged_name_falls_back_to_the_default() {
        let connected = [source("alsa_input.pci.analog-stereo")];
        assert_eq!(capture_source(Some(HEADSET), &connected), None);
    }

    #[test]
    fn an_unread_graph_trusts_the_name() {
        assert_eq!(
            capture_source(Some(HEADSET), &[]),
            Some(SourceId::new(HEADSET))
        );
    }
}
