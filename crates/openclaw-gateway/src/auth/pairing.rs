use dashmap::DashMap;
use uuid::Uuid;

pub struct PairingManager {
    codes: DashMap<String, Uuid>,
}

impl PairingManager {
    pub fn new() -> Self {
        Self {
            codes: DashMap::new(),
        }
    }

    pub fn generate_code(&self, session_id: Uuid) -> String {
        let code = Uuid::new_v4().to_string()[..6].to_uppercase();
        self.codes.insert(code.clone(), session_id);
        code
    }

    pub fn verify_code(&self, code: &str) -> Option<Uuid> {
        self.codes.remove(code).map(|(_, id)| id)
    }
}

impl Default for PairingManager {
    fn default() -> Self {
        Self::new()
    }
}
