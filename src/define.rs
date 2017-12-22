#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum ErrCode {
    Success = 0,
    DigestFailure = 1,
    FileErr = 2,
    JsonErr = 3,
    ConfigErr = 4,
    UrlErr = 5,
    SocketErr = 6,
    UnImplementErr = 7,
    KeyFmtErr = 8,
    NetErr = 9,

    UnDefined = 10000, //未知错误
}

impl From<ErrCode> for u64 {

    fn from(code:ErrCode) -> Self {
        code as u64 
    }
}

impl ErrCode {

    pub fn description(&self) -> &'static str {
        match *self {
            ErrCode::Success => "操作成功",
            ErrCode::DigestFailure => "加密检验失败",
            ErrCode::FileErr => "文件错误",
            ErrCode::JsonErr => "json错误",
            ErrCode::ConfigErr => "配置错误",
            ErrCode::UrlErr => "url错误",
            ErrCode::SocketErr => "socket错误",
            ErrCode::UnImplementErr => "所用功能未实现",
            ErrCode::KeyFmtErr => "键格式错误",
            ErrCode::NetErr => "网络错误",

            ErrCode::UnDefined => "未知错误",
        }
    }

    pub fn from_u64(code:u64) -> ErrCode {
        match code {
            0 => ErrCode::Success,
            1 => ErrCode::DigestFailure,
            2 => ErrCode::FileErr,
            3 => ErrCode::JsonErr,
            4 => ErrCode::ConfigErr,
            5 => ErrCode::UrlErr,
            6 => ErrCode::SocketErr,
            7 => ErrCode::UnImplementErr,
            8 => ErrCode::KeyFmtErr,
            9 => ErrCode::NetErr,

            _ => ErrCode::UnDefined,
        }
    }
}

extern crate futures;
use futures::Future;
use std::io;
pub type RunRst = Result<Box<Future<Item = (), Error = io::Error>>, ErrCode>;
