// SPDX-FileCopyrightText: © Matteo Settenvini <matteo.settenvini@montecristosoftware.eu>
// SPDX-License-Identifier: EUPL-1.2

mod common;

use scraper::{Html, Selector};

#[test_log::test(tokio::test)]
async fn serves_files() -> anyhow::Result<()> {
    let test = common::Test::new().await?;

    test.bucket
        .put_object("file.txt", "I am a file".as_bytes())
        .await?;
    test.bucket
        .put_object("folder/file.txt", "I am a file in a folder".as_bytes())
        .await?;

    let resp = reqwest::get(test.base_url.join("file.txt")?).await?;
    assert_eq!(resp.bytes().await?, "I am a file");

    let resp = reqwest::get(test.base_url.join("folder/file.txt")?).await?;
    assert_eq!(resp.bytes().await?, "I am a file in a folder");

    Ok(())
}

#[test_log::test(tokio::test)]
async fn serves_top_level_folder() -> anyhow::Result<()> {
    let test = common::Test::new().await?;

    test.bucket
        .put_object("file.txt", "I am a file".as_bytes())
        .await?;
    test.bucket
        .put_object("folder/file.txt", "I am a file in a folder".as_bytes())
        .await?;

    // Check that a file in the toplevel is listed:
    let resp = reqwest::get(test.base_url.clone()).await?;
    assert!(
        resp.status().is_success(),
        "Request failed with {}",
        resp.status()
    );
    let text = resp.text().await?;
    println!("{}", &text);
    let document = Html::parse_document(&text);

    let selector = Selector::parse(r#"h1"#).unwrap();
    for title in document.select(&selector) {
        assert_eq!(title.inner_html(), "/", "title doesn't match");
    }

    let selector =
        Selector::parse(r#"table > tbody > tr:nth-child(1) > td:first-child > a"#).unwrap();
    for item in document.select(&selector) {
        assert_eq!(item.attr("href"), Some("folder/"));
        assert_eq!(item.text().next(), Some("folder/"));
    }

    let selector =
        Selector::parse(r#"table > tbody > tr:nth-child(2) > td:first-child > a"#).unwrap();
    for item in document.select(&selector) {
        assert_eq!(item.attr("href"), Some("file.txt"));
        assert_eq!(item.text().next(), Some("file.txt"));
    }

    Ok(())
}

#[test_log::test(tokio::test)]
async fn serves_second_level_folder() -> anyhow::Result<()> {
    let test = common::Test::new().await?;

    test.bucket
        .put_object("file.txt", "I am a file".as_bytes())
        .await?;
    test.bucket
        .put_object("folder/file.txt", "I am a file in a folder".as_bytes())
        .await?;

    // Check that a file in the second level is listed:
    let resp = reqwest::get(test.base_url.join("folder/")?).await?;
    assert!(
        resp.status().is_success(),
        "Request failed with {}",
        resp.status()
    );
    let text = resp.text().await?;
    println!("{}", &text);
    let document = Html::parse_document(&text);

    let selector = Selector::parse(r#"h1"#).unwrap();
    for title in document.select(&selector) {
        assert_eq!(title.inner_html(), "folder/", "title doesn't match");
    }

    let selector =
        Selector::parse(r#"table > tbody > tr:nth-child(1) > td:first-child > a"#).unwrap();
    for item in document.select(&selector) {
        assert_eq!(item.attr("href"), Some("../"));
        assert_eq!(item.inner_html(), "..");
    }

    let selector =
        Selector::parse(r#"table > tbody > tr:nth-child(2) > td:first-child > a"#).unwrap();
    for item in document.select(&selector) {
        assert_eq!(item.attr("href"), Some("file.txt"));
        assert_eq!(item.inner_html(), "file.txt");
    }

    Ok(())
}

#[test_log::test(tokio::test)]
async fn serves_second_level_folder_without_ending_slash() -> anyhow::Result<()> {
    let test = common::Test::new().await?;

    test.bucket
        .put_object("file.txt", "I am a file".as_bytes())
        .await?;
    test.bucket
        .put_object("folder/file.txt", "I am a file in a folder".as_bytes())
        .await?;

    // Check that a file in the second level is listed even without an ending slash:
    let resp = reqwest::get(test.base_url.join("folder")?).await?;
    assert!(
        resp.status().is_success(),
        "Request failed with {}",
        resp.status()
    );
    let text = resp.text().await?;
    println!("{}", &text);
    let document = Html::parse_document(&text);

    let selector = Selector::parse(r#"h1"#).unwrap();
    for title in document.select(&selector) {
        assert_eq!(title.inner_html(), "folder/", "title doesn't match");
    }

    let selector =
        Selector::parse(r#"table > tbody > tr:nth-child(1) > td:first-child > a"#).unwrap();
    for item in document.select(&selector) {
        assert_eq!(item.attr("href"), Some("../"));
        assert_eq!(item.inner_html(), "..");
    }

    let selector =
        Selector::parse(r#"table > tbody > tr:nth-child(2) > td:first-child > a"#).unwrap();
    for item in document.select(&selector) {
        assert_eq!(item.attr("href"), Some("file.txt"));
        assert_eq!(item.inner_html(), "file.txt");
    }

    Ok(())
}
