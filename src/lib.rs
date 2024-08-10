/*！ 跳过command指令封装指令相应方法*/
use std::path::PathBuf;
use tokio::fs;
use tokio::task::JoinSet;
use crate::dep::command::search;
use crate::dep::model::HentaiStore;
use crate::dep::request::download_image;

mod dep;

/// 根据名称下载hentai
/// # args
///
/// * name-名称
/// * language-语言
/// * download_dir-下载目录
/// * replace_if_exist-目标存在时是否替换

async fn download(name:&str, language:&str, download_dir:&str, replace_if_exist:bool) -> anyhow::Result<()> {

    let base_url = "https://i3.nhentai.net/galleries";
    //搜索
    let hentai_detail = search(name.clone(), language).await?;
    //创建目录
    fs::create_dir_all(format!("{}/{}", download_dir.clone(), name.clone())).await?;
    //并发下载
    // 并发任务集合
    let mut set = JoinSet::new();
    for ele in hentai_detail.res_list {
        // let mut path = PathBuf::new();
        // path.push(config.root_dir.as_str());
        // path.push(name);
        // path.push(ele.as_str());
        let hentai_store = HentaiStore {
            url: format!("{}/{}/{}", base_url, &hentai_detail.gallery, ele),
            path: PathBuf::from(format!("{}/{}/{}", download_dir, name, ele)),
        };
        set.spawn(download_image(hentai_store, 0, replace_if_exist));
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    macro_rules! aw {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }
    
    use crate::download;
    #[test]
    fn download_test() {
        println!("{:?}",
                 aw!(
            download("触手落とし穴と女魔導士ちゃん","Chinese" ,"D:\\SoftwareLibrary\\code\\GitHubCode\\nhentai-rs\\target\\download", true)
        ));
    }
}