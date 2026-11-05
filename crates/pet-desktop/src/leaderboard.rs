//! Anonymous HTTPS client. Repository write credentials exist only on the server.
pub const ENABLED: bool = false;
use crate::game::Record;
use pet_core::preferences::atomic_write;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{Mutex, mpsc},
    time::Duration,
};
#[derive(Default, Serialize, Deserialize)]
pub struct Profile {
    pub nickname: String,
    pub publish: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Entry {
    pub nickname: String,
    pub hits: u32,
    pub timestamp: u64,
}
type Reply = Result<Vec<Entry>, String>;
pub struct Client {
    pub profile: Profile,
    pub entries: Vec<Entry>,
    pub status: String,
    pub endpoint: String,
    path: PathBuf,
    receiver: Option<Mutex<mpsc::Receiver<Reply>>>,
    pending: Option<Record>,
}
impl Client {
    pub fn load(data: &Path) -> Self {
        let path = data.join("practice-profile.json");
        let profile = std::fs::read(&path)
            .ok()
            .filter(|b| b.len() < 8192)
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self {
            profile,
            entries: vec![],
            status: String::new(),
            endpoint: option_env!("LPP_LEADERBOARD_URL")
                .unwrap_or("")
                .trim_end_matches('/')
                .into(),
            path,
            receiver: None,
            pending: None,
        }
    }
    pub fn save_profile(&mut self) {
        self.profile.nickname = self
            .profile
            .nickname
            .chars()
            .filter(|c| !c.is_control())
            .take(24)
            .collect();
        if let Err(e) = serde_json::to_vec(&self.profile)
            .map_err(std::io::Error::other)
            .and_then(|b| atomic_write(&self.path, &b))
        {
            self.status = format!("昵称保存失败：{e}");
        }
    }
    pub fn busy(&self) -> bool {
        self.receiver.is_some()
    }
    pub fn submit(&mut self, record: &Record) {
        if !ENABLED {
            return;
        }
        if !self.profile.publish || self.profile.nickname.trim().is_empty() {
            return;
        }
        self.pending = Some(record.clone());
        if !self.busy() {
            self.request(true);
        }
    }
    pub fn refresh(&mut self) {
        if !ENABLED {
            return;
        }
        if !self.busy() {
            self.request(false);
        }
    }
    fn request(&mut self, upload: bool) {
        if !self.endpoint.starts_with("https://") {
            self.status = "排行榜服务尚未配置；成绩已保存在本机。".into();
            self.pending = None;
            return;
        }
        let endpoint = self.endpoint.clone();
        let body = if upload {
            self.pending.take().map(|record| {
                serde_json::json!({
                    "protocol":1,"nickname":self.profile.nickname.trim(),"record":record
                })
                .to_string()
            })
        } else {
            None
        };
        let (send, recv) = mpsc::channel();
        self.receiver = Some(Mutex::new(recv));
        self.status = if body.is_some() {
            "正在提交成绩…"
        } else {
            "正在读取排行榜…"
        }
        .into();
        std::thread::spawn(move || {
            let result = (|| -> Reply {
                let agent: ureq::Agent = ureq::Agent::config_builder()
                    .timeout_global(Some(Duration::from_secs(15)))
                    .build()
                    .into();
                let mut response = if let Some(body) = body {
                    agent
                        .post(&format!("{endpoint}/scores"))
                        .header("Content-Type", "application/json")
                        .send(body.as_bytes())
                } else {
                    agent.get(&format!("{endpoint}/scores")).call()
                }
                .map_err(|_| {
                    "排行榜暂时不可用，或提交过于频繁；本地记录保留，可稍后重试。".to_owned()
                })?;
                let text = response
                    .body_mut()
                    .with_config()
                    .limit(128 * 1024)
                    .read_to_string()
                    .map_err(|_| "排行榜响应读取失败".to_owned())?;
                let entries: Vec<Entry> =
                    serde_json::from_str(&text).map_err(|_| "排行榜数据无效".to_owned())?;
                if entries.len() > 100
                    || entries
                        .iter()
                        .any(|e| e.nickname.chars().count() > 24 || e.hits > 750)
                {
                    return Err("排行榜数据无效".into());
                }
                Ok(entries)
            })();
            let _ = send.send(result);
        });
    }
    pub fn poll(&mut self) {
        let reply = self
            .receiver
            .as_ref()
            .and_then(|r| match r.lock().ok()?.try_recv() {
                Ok(v) => Some(v),
                Err(mpsc::TryRecvError::Disconnected) => Some(Err("排行榜连接已中断".into())),
                Err(mpsc::TryRecvError::Empty) => None,
            });
        if let Some(reply) = reply {
            self.receiver = None;
            match reply {
                Ok(entries) => {
                    self.entries = entries;
                    self.status = "排行榜已更新".into();
                }
                Err(e) => self.status = e,
            }
            if self.pending.is_some() {
                self.request(true);
            }
        }
    }
}
