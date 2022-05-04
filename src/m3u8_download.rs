use std::path::{PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct M3U8Download {
    url: String,
    path: PathBuf,
    thread: u8,
}

impl M3U8Download {
    /// 创建对象.
    pub fn from(url: String, output: String, thread: u8) -> M3U8Download {
        let path = PathBuf::from(&output);
        M3U8Download {
            url,
            // output,
            path,
            thread,
        }
    }
    /// 开始任务.
    pub async fn start(&self) -> Result<()> {
        // 检查目录
        self.check_directory().await?;
        // 下载索引文件
        let index_file = self.get_index().await?;
        // 解析任务
        let task_info = self.parse(&index_file).await?;
        // 秘钥
        if !task_info.0.is_empty() {
            self.download_key(&task_info.0).await?;
        }
        // 分片
        self.download_task(task_info.1, self.thread).await?;
        // 解析配置
        self.parse_index_m3u8().await?;
        Ok(())
    }
    /// 检查目录.
    async fn check_directory(&self) -> Result<()> {
        if self.path.exists() {
            return Ok(());
        }
        if self.path.is_dir() {
            return Ok(());
        }
        tokio::fs::create_dir_all(&self.path).await?;
        return Ok(());
    }
    /// 获取索引文件.
    async fn get_index(&self) -> Result<String> {
        let mut index_path = self.path.to_owned();
        index_path.push("index.m3u8");
        if index_path.exists() {
            return Ok(tokio::fs::read_to_string(index_path).await?);
        }
        let response = reqwest::get(&self.url)
            .await?.text().await?;
        if !response.starts_with("#EXTM3U") {
            panic!("{}:{}", "response error", &response);
        }
        tokio::fs::write(index_path, &response).await?;
        return Ok(response);
    }
    /// 解析index文件.
    async fn parse_index_m3u8(&self) -> Result<()> {
        let mut index_path = self.path.to_owned();
        index_path.push("index.m3u8");
        if !index_path.exists() {
            return Ok(());
        }
        let m3u8 = &tokio::fs::read_to_string(&index_path).await?.replace("\r\n", "\n");
        let rows: Vec<&str> = m3u8.split("\n").collect();
        let rows_count = rows.len();
        let mut result = Vec::with_capacity(rows_count);
        for row in rows.iter() {
            if row.starts_with("#EXT-X-KEY:") {
                let mut values: Vec<&str> = row.split(",").collect();
                values[1] = "URI=\"key.m3u8\"";
                result.push(values.join(","));
                continue;
            }
            if !row.starts_with("#") {
                let name = Self::parse_name(row).await;
                let mut path = self.path.to_owned();
                path.push(&name);
                if path.exists() {
                    result.push(name);
                    continue;
                }
            }
            result.push(row.to_string());
        }
        Ok(tokio::fs::write(index_path, &result.join("\n")).await?)
    }
    /// 拆解任务.
    async fn parse(&self, index_file: &str) -> Result<(String, Vec<String>)> {
        let index_file = index_file.replace("\r\n", "\n");
        let rows: Vec<&str> = index_file.split("\n").collect();
        let mut result: Vec<String> = vec![];
        let mut key = String::new();
        for row in rows.iter() {
            if row.starts_with("#EXT-X-KEY:") {
                let values: Vec<&str> = row.split(",").collect();
                for it in values.iter() {
                    if it.starts_with("URI=") {
                        key.push_str(&(*it).replace("URI=", "").replace("\"", ""));
                        break;
                    }
                }
            }
            if row.starts_with("#") {
                continue;
            }
            result.push(row.to_string());
        }
        Ok((key, result))
    }
    /// 下载秘钥.
    async fn download_key(&self, key_url: &str) -> Result<()> {
        let mut key_path = self.path.to_owned();
        key_path.push("key.m3u8");
        if key_path.exists() {
            return Ok(());
        }
        let response = reqwest::get(Self::parse_url(&self.url, key_url).await)
            .await?.bytes().await?;
        Ok(tokio::fs::write(key_path, &response).await?)
    }
    /// 下载分片流.
    async fn download_task(&self, task_list: Vec<String>, thread_count: u8) -> Result<()> {
        let lock_task_sub = Arc::new(Mutex::new(task_list));
        let lock_value_sub = Arc::new(Mutex::new(self.clone()));
        // 分块
        let mut handles = Vec::with_capacity(thread_count as usize);
        for _ in 0..thread_count {
            let lock_task = Arc::clone(&lock_task_sub);
            let lock_value = Arc::clone(&lock_value_sub);
            handles.push(tokio::spawn(async move {
                loop {
                    let mut task_list = lock_task.lock().await;
                    let value = lock_value.lock().await;
                    let item = task_list.pop();
                    match item {
                        None => return,
                        Some(task_url) => {
                            match Self::download_item(&value, task_url).await {
                                Ok(_) => {}
                                Err(error) => println!("{:?}", error)
                            }
                        }
                    }
                }
            }));
        }
        for handle in handles {
            handle.await?;
        }
        Ok(())
    }
    /// 解析URL地址.
    async fn parse_url(root_url: &str, url: &str) -> String {
        if url.starts_with("http") {
            return url.to_string();
        }
        let mut urls: Vec<&str> = root_url.split("/").collect();
        let urls_length = urls.len();
        urls[urls_length - 1] = url;
        return urls.join("/");
    }
    /// 解析URL获取文件名.
    async fn parse_name(value: &str) -> String {
        let urls: Vec<&str> = value.split("/").collect();
        let urls_length = urls.len();
        return urls[urls_length - 1].to_string();
    }
    /// 下载分片.
    async fn download_item(value: &M3U8Download, url: String) -> Result<()> {
        if url.is_empty() {
            return Ok(());
        }
        let mut ts_path = (&value.path).to_owned();
        ts_path.push(&Self::parse_name(&url).await);
        if ts_path.exists() {
            return Ok(());
        }
        println!("download >> {}", url);
        let download_url = Self::parse_url(&value.url, &url).await;
        loop {
            let response = reqwest::get(&download_url).await;
            match response {
                Ok(v) => {
                    return match v.bytes().await {
                        Ok(file) => {
                            tokio::fs::write(&ts_path, &file).await?;
                            Ok(())
                        }
                        Err(error) => {
                            println!("Response body error: {} {}", download_url, error);
                            Ok(())
                        }
                    };
                }
                Err(error) => {
                    println!("Request {}, {} fail !\nStart retry: {}", download_url, error, download_url);
                }
            };
        }
    }
}