extern crate csv;

use std::path::PathBuf;

use crate::detections::print::Message;
use crate::detections::rule;
use crate::detections::rule::RuleNode;
use crate::yaml::ParseYaml;

use evtx::err;
use evtx::{EvtxParser, SerializedEvtxRecord};
use serde_json::{Error, Value};

const DIRPATH_RULES: &str = "rules";

// TODO テストケースかかなきゃ...
#[derive(Debug)]
pub struct Detection {}

impl Detection {
    pub fn new() -> Detection {
        Detection {}
    }

    pub fn start(&mut self, evtx_files: Vec<PathBuf>) {
        if evtx_files.is_empty() {
            return;
        }

        // parse rule files
        let mut selection_rules = self.parse_rule_files();
        if selection_rules.is_empty() {
            return;
        }

        // serialize from evtx files to json
        let evtx_records = self.serialize_evtx_to_jsons(evtx_files);

        // select rule files and collect message
        let mut message = Message::new();
        selection_rules.iter_mut().for_each(|rule| {
            evtx_records.iter().for_each(|event_record| {
                if !rule.select(event_record) {
                    return;
                }

                message.insert(
                    event_record,
                    rule.yaml["output"].as_str().unwrap_or("").to_string(),
                )
            });
        });

        // output message
        message.print();
    }

    // serialize evtx files to json
    fn serialize_evtx_to_jsons(&self, evtx_files: Vec<PathBuf>) -> Vec<Value> {
        return evtx_files
            .iter()
            .filter_map(|evtx_file| {
                // convert to evtx parser
                match EvtxParser::from_path(evtx_file) {
                    Ok(parser) => Option::Some(parser),
                    Err(e) => {
                        eprintln!("{}", e);
                        return Option::None;
                    }
                }
            })
            .map(|mut cur| {
                let ret: Vec<err::Result<SerializedEvtxRecord<String>>> =
                    cur.records_json().collect();
                return ret;
            })
            .flatten()
            .filter_map(|json_record| {
                // convert from evtx parser to evtx json string records
                if json_record.is_ok() {
                    return Option::Some(json_record.unwrap());
                } else {
                    eprintln!("{}", json_record.unwrap_err());
                    return Option::None;
                }
            })
            .filter_map(|json_record| {
                // serialize json from json string
                let result_json: Result<Value, Error> = serde_json::from_str(&json_record.data); //// https://rust-lang-nursery.github.io/rust-cookbook/encoding/complex.html
                if result_json.is_err() {
                    eprintln!("{}", result_json.unwrap_err());
                    return Option::None;
                } else {
                    return result_json.ok();
                }
            })
            .collect();
    }

    fn parse_rule_files(&self) -> Vec<RuleNode> {
        // load rule files
        let mut rulefile_loader = ParseYaml::new();
        let resutl_readdir = rulefile_loader.read_dir(DIRPATH_RULES);
        if resutl_readdir.is_err() {
            eprintln!("{}", resutl_readdir.unwrap_err());
            return vec![];
        }

        // parse rule files
        return rulefile_loader
            .files
            .into_iter()
            .map(|rule_file| rule::parse_rule(rule_file))
            .filter_map(|mut rule| {
                let err_msgs_result = rule.init();
                if err_msgs_result.is_ok() {
                    return Option::Some(rule);
                }

                // ruleファイルの初期化失敗時のエラーを表示する部分
                err_msgs_result.err().iter().for_each(|err_msgs| {
                    // TODO 本当はファイルパスを出力したい
                    // ParseYamlの変更が必要なので、一旦yamlのタイトルを表示。

                    // TODO エラーの出力方法を統一したい。
                    // エラー出力用のクラスを作成してもいいかも
                    println!(
                        "[ERROR] Failed to parse Rule file. (Error Rule Title : {})",
                        rule.yaml["title"].as_str().unwrap_or("")
                    );
                    err_msgs.iter().for_each(|err_msg| println!("{}", err_msg));
                    println!("");
                });

                return Option::None;
            })
            .collect();
    }
}
