pub struct MachineInfo {
    pub system: String,
    pub cpu: String,
    pub endian: String,
}

pub trait Os: 'static {
    // env
    fn get_env(&self, key: &str) -> Option<String>;
    fn host(&self) -> MachineInfo;
    fn target(&self) -> MachineInfo;

    // path
    fn join_paths(&self, paths: &[&str]) -> String;

    // fs
    fn is_file(&self, path: &str) -> Result<bool, String>;
    fn is_dir(&self, path: &str) -> Result<bool, String>;
    fn exists(&self, path: &str) -> Result<bool, String>;
    fn read_file(&self, path: &str) -> Result<Vec<u8>, String>;

    // compiler
    fn get_compiler(&self, lang: &str) -> Result<Vec<String>, String>;
}
