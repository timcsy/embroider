use stitch_core::UserMessage;

struct Entry {
    id: String,
    name: String,
    data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontInfo {
    pub id: String,
    pub name: String,
}

/// 已載入的字型集合。順序即退回（fallback）順序。
#[derive(Default)]
pub struct FontSet {
    entries: Vec<Entry>,
}

impl FontSet {
    /// 加入字型；資料不是合法字型時回傳白話錯誤，集合不變。
    pub fn add(&mut self, id: &str, name: &str, data: Vec<u8>) -> Result<(), UserMessage> {
        if ttf_parser::Face::parse(&data, 0).is_err() {
            return Err(UserMessage::new(
                "bad_font",
                "這個字型檔打不開。",
                Some("請換一個字型，或重新整理頁面再試一次。"),
            ));
        }
        self.entries.retain(|e| e.id != id);
        self.entries.push(Entry {
            id: id.to_string(),
            name: name.to_string(),
            data,
        });
        Ok(())
    }

    pub fn list(&self) -> Vec<FontInfo> {
        self.entries
            .iter()
            .map(|e| FontInfo {
                id: e.id.clone(),
                name: e.name.clone(),
            })
            .collect()
    }

    pub(crate) fn index_of(&self, id: &str) -> Option<usize> {
        self.entries.iter().position(|e| e.id == id)
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn face(&self, index: usize) -> ttf_parser::Face<'_> {
        // add() 已驗證過資料，這裡不會失敗
        ttf_parser::Face::parse(&self.entries[index].data, 0).expect("已驗證的字型")
    }
}
