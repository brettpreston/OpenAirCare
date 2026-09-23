//! OpenAirCare: a Makepad desktop UI over `airpods-link`.
//!
//! UI -> session: `SessionHandle::send(Command)`.
//! Session -> UI: the session thread posts `SessionEvent`s with
//! `Cx::post_action`; `handle_actions` downcasts them and refreshes widgets.

pub use makepad_widgets;

mod settings;

use std::sync::Arc;

use airpods_link::session::{Command, ConnectionState, SessionEvent, SessionHandle, Snapshot};
use airpods_link::Transport;
use airpods_proto::hearing::{Adjustments, Audiogram, BANDS_HZ};
use airpods_proto::model;
use airpods_proto::ListeningMode;
use makepad_widgets::*;

use crate::settings::Settings;

app_main!(App);

script_mod! {
    use mod.prelude.widgets.*

    // The theme draws text as white at ~35% alpha (`color_u_5`), which reads as
    // grey on the dark background above. Widget prototypes bake their colour in
    // when the prelude is built, so overriding `theme.color_text` afterwards has
    // no effect -- restyle the types this UI actually uses instead.
    // `color_disabled` and `color_empty` (placeholder) are left dimmer on purpose,
    // so greyed-out controls and empty fields still read as such.
    let Label = Label{draw_text +: {color: #FFFFFFFF}}
    let H3 = H3{draw_text +: {color: #FFFFFFFF}}
    let Button = Button{draw_text +: {
        color: #FFFFFFFF color_hover: #FFFFFFFF color_down: #FFFFFFFF color_focus: #FFFFFFFF
    }}
    let TextInput = TextInput{draw_text +: {
        color: #FFFFFFFF color_hover: #FFFFFFFF color_focus: #FFFFFFFF color_down: #FFFFFFFF
    }}
    let CheckBox = CheckBox{draw_text +: {
        color: #FFFFFFFF color_hover: #FFFFFFFF color_down: #FFFFFFFF
        color_focus: #FFFFFFFF color_active: #FFFFFFFF
    }}
    let Toggle = Toggle{draw_text +: {
        color: #FFFFFFFF color_hover: #FFFFFFFF color_down: #FFFFFFFF
        color_focus: #FFFFFFFF color_active: #FFFFFFFF
    }}
    let RadioButtonTab = RadioButtonTab{draw_text +: {
        color: #FFFFFFFF color_hover: #FFFFFFFF color_down: #FFFFFFFF
        color_focus: #FFFFFFFF color_active: #FFFFFFFF
    }}
    // The slider's own label plus the editable value field inside it.
    let Slider = Slider{
        draw_text +: {
            color: #FFFFFFFF color_hover: #FFFFFFFF color_drag: #FFFFFFFF color_focus: #FFFFFFFF
        }
        text_input +: {draw_text +: {
            color: #FFFFFFFF color_hover: #FFFFFFFF color_focus: #FFFFFFFF color_down: #FFFFFFFF
        }}
    }

    let app = startup() do #(App::script_component(vm)){
        ui: Root{
            main_window := Window{
                window.title: "OpenAirCare"
                window.inner_size: vec2(600, 820)
                // Darker than the theme's default `color_bg_app` (a mid grey).
                pass +: { clear_color: #141414 }
                body +: {
                    View{
                        width: Fill height: Fill
                        flow: Down
                        padding: 12
                        spacing: 10

                        tabs := View{
                            width: Fit height: Fit
                            flow: Right
                            spacing: theme.space_2
                            tab_status := RadioButtonTab{text: "Status"}
                            tab_hearing := RadioButtonTab{text: "Hearing Aid"}
                            tab_audiogram := RadioButtonTab{text: "Audiogram"}
                            tab_adjust := RadioButtonTab{text: "Adjustments"}
                        }

                        pages := PageFlip{
                            width: Fill height: Fill
                            flow: Down
                            active_page: @page_status

                            // ---------------------------------------------------- Status
                            page_status := ScrollYView{
                                width: Fill height: Fill
                                flow: Down
                                spacing: 8

                                H3{text: "Connection"}
                                state_label := Label{width: Fill text: "Disconnected"}
                                error_label := Label{width: Fill text: ""}
                                device_label := Label{width: Fill text: "No device"}
                                battery_label := Label{width: Fill text: ""}
                                View{
                                    width: Fill height: Fit
                                    flow: Right
                                    spacing: 8
                                    connect_btn := Button{text: "Connect"}
                                    disconnect_btn := Button{text: "Disconnect"}
                                    refresh_btn := Button{text: "List devices"}
                                }
                                Label{text: "Device MAC (leave empty for the first paired AirPods)"}
                                mac_input := TextInput{
                                    width: Fill height: 36
                                    empty_text: "XX:XX:XX:XX:XX:XX"
                                    autocorrect: Disabled
                                    autocapitalize: None
                                }
                                auto_reconnect_check := CheckBox{text: "Reconnect automatically", active: true}

                                Hr{}
                                H3{text: "Listening mode"}
                                Label{width: Fill text: "Hearing Aid only works in Transparency mode."}
                                modes := View{
                                    width: Fit height: Fit
                                    flow: Right
                                    spacing: theme.space_2
                                    mode_off := RadioButtonTab{text: "Off"}
                                    mode_anc := RadioButtonTab{text: "Noise Cancellation"}
                                    mode_transparency := RadioButtonTab{text: "Transparency"}
                                    mode_adaptive := RadioButtonTab{text: "Adaptive"}
                                }

                                Hr{}
                                H3{text: "Log"}
                                log_label := Label{width: Fill text: ""}
                            }

                            // ---------------------------------------------------- Hearing aid
                            page_hearing := ScrollYView{
                                width: Fill height: Fill
                                flow: Down
                                spacing: 10

                                H3{text: "Hearing Aid"}
                                capability_label := Label{width: Fill text: "Connect your AirPods first."}
                                ha_toggle := Toggle{text: "Hearing Aid enabled"}
                                swipe_toggle := Toggle{text: "Swipe the stem to adjust amplification"}
                                Hr{}
                                att_label := Label{width: Fill text: ""}
                                Label{
                                    width: Fill
                                    text: "Enabling Hearing Aid turns off Customized Transparency and Headphone Accommodation on the AirPods. Use the Audiogram tab to load your hearing test results, then fine tune on the Adjustments tab."
                                }
                            }

                            // ---------------------------------------------------- Audiogram
                            page_audiogram := ScrollYView{
                                width: Fill height: Fill
                                flow: Down
                                spacing: 8

                                H3{text: "Audiogram (hearing loss in dB HL)"}
                                Label{width: Fill text: "Enter the values from a professional hearing test. 0 = no loss. AirPods use bands 250 Hz to 8 kHz."}
                                View{
                                    width: Fill height: Fit flow: Right spacing: 8
                                    Label{width: 80 text: "Band"}
                                    Label{width: 100 text: "Left"}
                                    Label{width: 100 text: "Right"}
                                }
                                View{ width: Fill height: Fit flow: Right spacing: 8
                                    Label{width: 80 text: "250 Hz"}
                                    ag_l_0 := TextInput{width: 100 height: 32 is_numeric_only: true empty_text: "0"}
                                    ag_r_0 := TextInput{width: 100 height: 32 is_numeric_only: true empty_text: "0"}
                                }
                                View{ width: Fill height: Fit flow: Right spacing: 8
                                    Label{width: 80 text: "500 Hz"}
                                    ag_l_1 := TextInput{width: 100 height: 32 is_numeric_only: true empty_text: "0"}
                                    ag_r_1 := TextInput{width: 100 height: 32 is_numeric_only: true empty_text: "0"}
                                }
                                View{ width: Fill height: Fit flow: Right spacing: 8
                                    Label{width: 80 text: "1 kHz"}
                                    ag_l_2 := TextInput{width: 100 height: 32 is_numeric_only: true empty_text: "0"}
                                    ag_r_2 := TextInput{width: 100 height: 32 is_numeric_only: true empty_text: "0"}
                                }
                                View{ width: Fill height: Fit flow: Right spacing: 8
                                    Label{width: 80 text: "2 kHz"}
                                    ag_l_3 := TextInput{width: 100 height: 32 is_numeric_only: true empty_text: "0"}
                                    ag_r_3 := TextInput{width: 100 height: 32 is_numeric_only: true empty_text: "0"}
                                }
                                View{ width: Fill height: Fit flow: Right spacing: 8
                                    Label{width: 80 text: "3 kHz"}
                                    ag_l_4 := TextInput{width: 100 height: 32 is_numeric_only: true empty_text: "0"}
                                    ag_r_4 := TextInput{width: 100 height: 32 is_numeric_only: true empty_text: "0"}
                                }
                                View{ width: Fill height: Fit flow: Right spacing: 8
                                    Label{width: 80 text: "4 kHz"}
                                    ag_l_5 := TextInput{width: 100 height: 32 is_numeric_only: true empty_text: "0"}
                                    ag_r_5 := TextInput{width: 100 height: 32 is_numeric_only: true empty_text: "0"}
                                }
                                View{ width: Fill height: Fit flow: Right spacing: 8
                                    Label{width: 80 text: "6 kHz"}
                                    ag_l_6 := TextInput{width: 100 height: 32 is_numeric_only: true empty_text: "0"}
                                    ag_r_6 := TextInput{width: 100 height: 32 is_numeric_only: true empty_text: "0"}
                                }
                                View{ width: Fill height: Fit flow: Right spacing: 8
                                    Label{width: 80 text: "8 kHz"}
                                    ag_l_7 := TextInput{width: 100 height: 32 is_numeric_only: true empty_text: "0"}
                                    ag_r_7 := TextInput{width: 100 height: 32 is_numeric_only: true empty_text: "0"}
                                }
                                View{
                                    width: Fill height: Fit flow: Right spacing: 8
                                    ag_apply_btn := Button{text: "Apply to AirPods"}
                                    ag_reload_btn := Button{text: "Reload from AirPods"}
                                }
                                View{
                                    width: Fill height: Fit flow: Right spacing: 8
                                    ag_save_btn := Button{text: "Save locally"}
                                    ag_load_btn := Button{text: "Load saved"}
                                }
                                ag_status_label := Label{width: Fill text: ""}
                            }

                            // ---------------------------------------------------- Adjustments
                            page_adjust := ScrollYView{
                                width: Fill height: Fill
                                flow: Down
                                spacing: 12

                                H3{text: "Adjustments"}
                                adj_hint_label := Label{width: Fill text: ""}
                                amp_slider := Slider{text: "Amplification  (1.00 = Apple's maximum; up to 2.00 works on the buds)" min: -1.0 max: 2.0 step: 0.01 precision: 2 default: 0.0}
                                bal_slider := Slider{text: "Balance  (left  <->  right)" min: -1.0 max: 1.0 step: 0.01 precision: 2 default: 0.0}
                                tone_slider := Slider{text: "Tone  (darker  <->  brighter)" min: -1.0 max: 1.0 step: 0.01 precision: 2 default: 0.0}
                                anr_slider := Slider{text: "Ambient noise reduction" min: 0.0 max: 1.0 step: 0.01 precision: 2 default: 0.0}
                                conv_check := CheckBox{text: "Conversation boost"}
                                own_voice_label := Label{width: Fill text: ""}
                                reset_btn := Button{text: "Reset adjustments"}
                            }
                        }
                    }
                }
            }
        }
    }
    app
}

const MAX_LOG_LINES: usize = 14;

#[derive(Script, ScriptHook)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[rust]
    session: Option<SessionHandle>,
    #[rust]
    snap: Option<Snapshot>,
    #[rust]
    logs: Vec<String>,
    #[rust]
    settings: Settings,
    /// The user edited audiogram fields since the last device sync.
    #[rust]
    audiogram_dirty: bool,
}

fn left_ids() -> [&'static [LiveId]; 8] {
    [
        ids!(ag_l_0), ids!(ag_l_1), ids!(ag_l_2), ids!(ag_l_3),
        ids!(ag_l_4), ids!(ag_l_5), ids!(ag_l_6), ids!(ag_l_7),
    ]
}

fn right_ids() -> [&'static [LiveId]; 8] {
    [
        ids!(ag_r_0), ids!(ag_r_1), ids!(ag_r_2), ids!(ag_r_3),
        ids!(ag_r_4), ids!(ag_r_5), ids!(ag_r_6), ids!(ag_r_7),
    ]
}

fn make_transport() -> Arc<dyn Transport> {
    #[cfg(feature = "mock")]
    {
        ::log::info!("using the mock AirPods transport");
        return Arc::new(airpods_link::mock::MockTransport::new());
    }
    #[allow(unreachable_code)]
    airpods_link::default_transport()
        .expect("no Bluetooth transport on this platform; build with `--features mock` for development")
}

impl App {
    fn send(&self, cmd: Command) {
        if let Some(s) = &self.session {
            s.send(cmd);
        }
    }

    fn push_log(&mut self, cx: &mut Cx, line: String) {
        self.logs.push(line);
        if self.logs.len() > MAX_LOG_LINES {
            let drop = self.logs.len() - MAX_LOG_LINES;
            self.logs.drain(..drop);
        }
        let text = self.logs.join("\n");
        self.ui.label(cx, ids!(log_label)).set_text(cx, &text);
    }

    fn mac_from_input(&self, cx: &mut Cx) -> Option<String> {
        let t = self.ui.text_input(cx, ids!(mac_input)).text();
        let t = t.trim().to_uppercase();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    }

    fn adjustments_from_ui(&self, cx: &mut Cx) -> Adjustments {
        let v = |id: &[LiveId]| self.ui.slider(cx, id).value().unwrap_or(0.0) as f32;
        Adjustments {
            amplification: v(ids!(amp_slider)),
            balance: v(ids!(bal_slider)),
            tone: v(ids!(tone_slider)),
            ambient_noise_reduction: v(ids!(anr_slider)),
            conversation_boost: self.ui.check_box(cx, ids!(conv_check)).active(cx),
        }
    }

    fn audiogram_from_ui(&self, cx: &mut Cx) -> Result<Audiogram, String> {
        let mut ag = Audiogram::default();
        for (i, (l, r)) in left_ids().iter().zip(right_ids().iter()).enumerate() {
            ag.left[i] = parse_db(&self.ui.text_input(cx, l).text()).map_err(|e| format!("left {} Hz: {e}", BANDS_HZ[i]))?;
            ag.right[i] = parse_db(&self.ui.text_input(cx, r).text()).map_err(|e| format!("right {} Hz: {e}", BANDS_HZ[i]))?;
        }
        Ok(ag)
    }

    fn audiogram_to_ui(&mut self, cx: &mut Cx, ag: &Audiogram) {
        for (i, (l, r)) in left_ids().iter().zip(right_ids().iter()).enumerate() {
            self.ui.text_input(cx, l).set_text(cx, &format_db(ag.left[i]));
            self.ui.text_input(cx, r).set_text(cx, &format_db(ag.right[i]));
        }
        self.audiogram_dirty = false;
    }

    fn adjustments_to_ui(&mut self, cx: &mut Cx, a: &Adjustments) {
        self.ui.slider(cx, ids!(amp_slider)).set_value(cx, a.amplification as f64);
        self.ui.slider(cx, ids!(bal_slider)).set_value(cx, a.balance as f64);
        self.ui.slider(cx, ids!(tone_slider)).set_value(cx, a.tone as f64);
        self.ui.slider(cx, ids!(anr_slider)).set_value(cx, a.ambient_noise_reduction as f64);
        self.ui.check_box(cx, ids!(conv_check)).set_active(cx, a.conversation_boost, Animate::No);
    }

    fn set_mode_radios(&mut self, cx: &mut Cx, mode: Option<ListeningMode>) {
        let ids: [(&[LiveId], ListeningMode); 4] = [
            (ids!(mode_off), ListeningMode::Off),
            (ids!(mode_anc), ListeningMode::NoiseCancellation),
            (ids!(mode_transparency), ListeningMode::Transparency),
            (ids!(mode_adaptive), ListeningMode::Adaptive),
        ];
        for (id, m) in ids {
            self.ui.radio_button(cx, id).set_active(cx, mode == Some(m), Animate::No);
        }
    }

    /// Refresh every widget from the latest snapshot.
    fn sync_ui(&mut self, cx: &mut Cx) {
        let Some(snap) = self.snap.clone() else { return };

        let state_text = match &snap.state {
            ConnectionState::Disconnected => "Disconnected".to_string(),
            ConnectionState::Connecting(stage) => format!("Connecting: {stage}"),
            ConnectionState::Connected => format!("Connected via {}", snap.backend),
            ConnectionState::Reconnecting { attempt, in_secs } => {
                format!("Retrying in {in_secs}s (attempt {attempt})")
            }
            ConnectionState::Error(e) => format!("Error: {e}"),
        };
        self.ui.label(cx, ids!(state_label)).set_text(cx, &state_text);
        let error_text = match (&snap.state, &snap.last_error) {
            (ConnectionState::Error(_), _) | (_, None) => String::new(),
            (_, Some(e)) => format!("Last error: {e}"),
        };
        self.ui.label(cx, ids!(error_label)).set_text(cx, &error_text);

        let device_text = match (&snap.device, &snap.info) {
            (Some(d), Some(i)) => format!(
                "{} [{}] - {} {} - firmware {}",
                i.name,
                d.mac,
                model::family_name(&i.model_number),
                i.model_number,
                i.firmware_version
            ),
            (Some(d), None) => format!("{} [{}]", d.name, d.mac),
            _ => "No device".to_string(),
        };
        self.ui.label(cx, ids!(device_label)).set_text(cx, &device_text);

        let battery_text = if snap.battery.is_empty() {
            String::new()
        } else {
            snap.battery
                .iter()
                .map(|b| {
                    use airpods_proto::aacp::battery::BatteryStatus;
                    match b.status {
                        BatteryStatus::Charging => format!("{} {}% (charging)", b.component.label(), b.level),
                        // Case lid closed / bud not in the case: the level byte is meaningless.
                        BatteryStatus::Disconnected => format!("{} --", b.component.label()),
                        _ => format!("{} {}%", b.component.label(), b.level),
                    }
                })
                .collect::<Vec<_>>()
                .join("   ")
        };
        self.ui.label(cx, ids!(battery_label)).set_text(cx, &battery_text);

        let connected = matches!(snap.state, ConnectionState::Connected);
        self.ui.button(cx, ids!(connect_btn)).set_enabled(cx, !connected);
        self.ui.button(cx, ids!(disconnect_btn)).set_enabled(cx, snap.state != ConnectionState::Disconnected);

        self.set_mode_radios(cx, snap.listening_mode);

        // Hearing aid page
        let capability_text = match (connected, snap.hearing_aid_capable, &snap.info) {
            (false, _, _) => "Connect your AirPods first (Status tab).".to_string(),
            (true, Some(true), Some(i)) => format!("{} ({}) supports Hearing Aid.", model::family_name(&i.model_number), i.model_number),
            (true, Some(false), Some(i)) => format!("{} ({}) does not support Hearing Aid (AirPods Pro 2 / Pro 3 only).", i.name, i.model_number),
            (true, _, _) => "Waiting for device information...".to_string(),
        };
        self.ui.label(cx, ids!(capability_label)).set_text(cx, &capability_text);
        self.ui.check_box(cx, ids!(ha_toggle)).set_active(cx, snap.hearing_aid_enabled.unwrap_or(false), Animate::No);
        self.ui.check_box(cx, ids!(swipe_toggle)).set_active(cx, snap.gain_swipe.unwrap_or(false), Animate::No);
        let ha_text = match (&snap.hearing_aid_enabled, snap.hearing_aid_enrolled) {
            (Some(true), _) => "Hearing Aid is ON.",
            (Some(false), true) => "Hearing Aid is off (audiogram enrolled).",
            (Some(false), false) => "Hearing Aid is off.",
            (None, _) => "Hearing Aid state unknown yet.",
        };
        self.ui.check_box(cx, ids!(ha_toggle)).set_text(&format!("Hearing Aid enabled - {ha_text}"));

        let att_text = if !connected {
            String::new()
        } else if snap.att_ok {
            match snap.data {
                Some(_) => "Hearing-aid settings channel (ATT) open; audiogram and adjustments are live.".to_string(),
                None => "ATT channel open, waiting for hearing-aid data...".to_string(),
            }
        } else {
            format!(
                "Hearing-aid settings channel unavailable: {}\n\nChecklist: add `DeviceID = bluetooth:004C:0000:0000` to /etc/bluetooth/main.conf, restart bluetooth, re-pair the AirPods, and make sure no other Pods app is running.",
                snap.att_error.clone().unwrap_or_default()
            )
        };
        self.ui.label(cx, ids!(att_label)).set_text(cx, &att_text);
        let audiogram_is_flat_zero = snap
            .data
            .map(|d| d.left.eq.iter().chain(d.right.eq.iter()).all(|v| v.abs() < 0.5))
            .unwrap_or(false);
        let adj_hint = if !(snap.att_ok && snap.data.is_some()) {
            "Not available until the hearing-aid settings channel is open (see Hearing Aid tab)."
        } else if audiogram_is_flat_zero {
            "The audiogram on the AirPods is all zeros (no hearing loss), so amplification has nothing to amplify: enter your audiogram first (Audiogram tab). Changes are written as you move the sliders."
        } else if snap.hearing_aid_enabled != Some(true) {
            "Changes are written as you move the sliders, but you will only hear them once Hearing Aid is on (Hearing Aid tab) and the buds are in Transparency mode."
        } else {
            "Changes are written to the AirPods as you move the sliders."
        };
        self.ui.label(cx, ids!(adj_hint_label)).set_text(cx, adj_hint);

        if let Some(d) = snap.data {
            self.adjustments_to_ui(cx, &d.adjustments());
            self.ui
                .label(cx, ids!(own_voice_label))
                .set_text(cx, &format!("Own voice amplification (device): {:.2}", d.own_voice_amplification));
            if !self.audiogram_dirty {
                self.audiogram_to_ui(cx, &d.audiogram());
            }
        }

        self.ui.check_box(cx, ids!(auto_reconnect_check)).set_active(cx, snap.auto_reconnect, Animate::No);
        self.ui.redraw(cx);
    }
}

fn parse_db(s: &str) -> Result<f32, String> {
    let t = s.trim();
    if t.is_empty() {
        return Ok(0.0);
    }
    t.parse::<f32>().map_err(|_| format!("'{t}' is not a number"))
}

fn format_db(v: f32) -> String {
    if (v - v.round()).abs() < 1e-3 {
        format!("{}", v.round() as i32)
    } else {
        format!("{v:.1}")
    }
}

impl MatchEvent for App {
    fn handle_startup(&mut self, cx: &mut Cx) {
        let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).try_init();
        self.settings = Settings::load();

        self.ui.radio_button(cx, ids!(tab_status)).set_active(cx, true, Animate::No);
        if let Some(mac) = &self.settings.device_mac {
            self.ui.text_input(cx, ids!(mac_input)).set_text(cx, mac);
        }
        if let Some(ag) = &self.settings.audiogram.clone() {
            self.audiogram_to_ui(cx, ag);
        }

        let emit: airpods_link::session::Emit = Arc::new(|e: SessionEvent| Cx::post_action(e));
        let handle = airpods_link::session::spawn(make_transport(), emit);
        if let Some(auto) = self.settings.auto_reconnect {
            handle.send(Command::SetAutoReconnect(auto));
        }
        handle.send(Command::Connect(self.settings.device_mac.clone()));
        self.session = Some(handle);
        self.push_log(cx, "started".into());
    }

    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        // ---- events from the session thread ----
        let mut got_snapshot = false;
        for action in actions {
            if let Some(ev) = action.downcast_ref::<SessionEvent>() {
                match ev {
                    SessionEvent::Snapshot(s) => {
                        self.snap = Some(s.clone());
                        got_snapshot = true;
                    }
                    SessionEvent::Log(l) => {
                        let line = l.clone();
                        self.push_log(cx, line);
                    }
                }
            }
        }
        if got_snapshot {
            self.sync_ui(cx);
        }

        // ---- tabs ----
        if let Some(i) = self
            .ui
            .radio_button_set(cx, ids_list!(tab_status, tab_hearing, tab_audiogram, tab_adjust))
            .selected(cx, actions)
        {
            let page = [live_id!(page_status), live_id!(page_hearing), live_id!(page_audiogram), live_id!(page_adjust)][i];
            self.ui.page_flip(cx, ids!(pages)).set_active_page(cx, page);
            self.ui.redraw(cx);
        }

        // ---- status page ----
        if self.ui.button(cx, ids!(connect_btn)).clicked(actions) {
            let mac = self.mac_from_input(cx);
            self.settings.device_mac = mac.clone();
            self.settings.save();
            self.send(Command::Connect(mac));
        }
        if self.ui.button(cx, ids!(disconnect_btn)).clicked(actions) {
            self.send(Command::Disconnect);
        }
        if self.ui.button(cx, ids!(refresh_btn)).clicked(actions) {
            self.send(Command::RefreshDevices);
        }
        if let Some(v) = self.ui.check_box(cx, ids!(auto_reconnect_check)).changed(actions) {
            self.settings.auto_reconnect = Some(v);
            self.settings.save();
            self.send(Command::SetAutoReconnect(v));
        }
        if let Some(i) = self
            .ui
            .radio_button_set(cx, ids_list!(mode_off, mode_anc, mode_transparency, mode_adaptive))
            .selected(cx, actions)
        {
            self.send(Command::SetListeningMode(ListeningMode::ALL[i]));
        }

        // ---- hearing aid page ----
        if let Some(on) = self.ui.check_box(cx, ids!(ha_toggle)).changed(actions) {
            self.send(Command::SetHearingAid(on));
        }
        if let Some(on) = self.ui.check_box(cx, ids!(swipe_toggle)).changed(actions) {
            self.send(Command::SetGainSwipe(on));
        }

        // ---- audiogram page ----
        for id in left_ids().iter().chain(right_ids().iter()) {
            if self.ui.text_input(cx, id).changed(actions).is_some() {
                self.audiogram_dirty = true;
            }
        }
        if self.ui.button(cx, ids!(ag_apply_btn)).clicked(actions) {
            match self.audiogram_from_ui(cx) {
                Ok(ag) => {
                    self.settings.audiogram = Some(ag);
                    self.settings.save();
                    self.audiogram_dirty = false;
                    self.send(Command::SetAudiogram(ag));
                    self.ui.label(cx, ids!(ag_status_label)).set_text(cx, "Audiogram sent to AirPods (and saved locally).");
                }
                Err(e) => self.ui.label(cx, ids!(ag_status_label)).set_text(cx, &format!("Invalid value: {e}")),
            }
        }
        if self.ui.button(cx, ids!(ag_reload_btn)).clicked(actions) {
            self.audiogram_dirty = false;
            self.send(Command::Reload);
            self.ui.label(cx, ids!(ag_status_label)).set_text(cx, "Reloading from AirPods...");
        }
        if self.ui.button(cx, ids!(ag_save_btn)).clicked(actions) {
            match self.audiogram_from_ui(cx) {
                Ok(ag) => {
                    self.settings.audiogram = Some(ag);
                    self.settings.save();
                    self.ui
                        .label(cx, ids!(ag_status_label))
                        .set_text(cx, &format!("Saved to {}", settings::settings_path().display()));
                }
                Err(e) => self.ui.label(cx, ids!(ag_status_label)).set_text(cx, &format!("Invalid value: {e}")),
            }
        }
        if self.ui.button(cx, ids!(ag_load_btn)).clicked(actions) {
            match self.settings.audiogram {
                Some(ag) => {
                    self.audiogram_to_ui(cx, &ag);
                    self.audiogram_dirty = true; // keep until applied
                    self.ui.label(cx, ids!(ag_status_label)).set_text(cx, "Loaded saved audiogram; press Apply to send it.");
                }
                None => self.ui.label(cx, ids!(ag_status_label)).set_text(cx, "No saved audiogram yet."),
            }
        }

        // ---- adjustments page ----
        let slid = [ids!(amp_slider), ids!(bal_slider), ids!(tone_slider), ids!(anr_slider)]
            .iter()
            .any(|id| self.ui.slider(cx, *id).slided(actions).is_some());
        let conv_changed = self.ui.check_box(cx, ids!(conv_check)).changed(actions).is_some();
        if slid || conv_changed {
            let adj = self.adjustments_from_ui(cx);
            self.settings.adjustments = Some(adj);
            self.send(Command::SetAdjustments(adj));
        }
        if self.ui.button(cx, ids!(reset_btn)).clicked(actions) {
            self.adjustments_to_ui(cx, &Adjustments::RESET);
            self.settings.adjustments = Some(Adjustments::RESET);
            self.settings.save();
            self.send(Command::ResetAdjustments);
        }
    }
}

impl AppMain for App {
    fn script_mod(vm: &mut ScriptVm) -> ScriptValue {
        crate::makepad_widgets::script_mod(vm);
        self::script_mod(vm)
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::Shutdown = event {
            self.settings.save();
        }
        self.match_event(cx, event);
        self.ui.handle_event(cx, event, &mut Scope::empty());
    }
}
