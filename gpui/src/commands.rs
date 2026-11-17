#[derive(Clone, Copy)]
pub enum Command { Open, Refresh, Save, SaveLayout, Import, Export, Reset, Create, Settings, Close }
impl Command {
 pub const ALL: &[Self]=&[Self::Open,Self::Refresh,Self::Save,Self::SaveLayout,Self::Import,Self::Export,Self::Reset,Self::Create,Self::Settings,Self::Close];
 pub fn label(self)-> &'static str {match self {Self::Open=>"Open workspace",Self::Refresh=>"Refresh workspace",Self::Save=>"Save document",Self::SaveLayout=>"Save layout",Self::Import=>"Import layout",Self::Export=>"Export layout",Self::Reset=>"Reset layout",Self::Create=>"New Skill",Self::Settings=>"Settings",Self::Close=>"Close command palette"}}
 pub fn needs_workspace(self)->bool {!matches!(self,Self::Open|Self::Settings|Self::Close)}
}
