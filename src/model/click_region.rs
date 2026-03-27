use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct Region {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl Region {
    pub fn contains(&self, col: u16, row: u16) -> bool {
        col >= self.x
            && col < self.x.saturating_add(self.w)
            && row >= self.y
            && row < self.y.saturating_add(self.h)
    }
}

#[derive(Clone, Debug)]
pub enum ClickAction {
    Dashboard(crate::tui::dashboard::DashboardAction),
    Keys(crate::tui::keys::KeyAction),
    Pin(crate::tui::pin::PinAction),
    Piv(crate::tui::piv::PivAction),
    Ssh(crate::tui::ssh::SshAction),
    Diagnostics(crate::tui::diagnostics::DiagnosticsAction),
    Help(crate::tui::help::HelpAction),
}

#[derive(Clone, Debug)]
pub struct ClickRegion {
    pub region: Region,
    pub action: ClickAction,
}
