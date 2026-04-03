use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u64 = 1;
pub const PREFIX_LENGTH_BYTES: usize = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageKind {
    /// 客户端: 连接请求
    ConnectionRequest,

    /// 服务器: 请求密码
    PasswordRequest,

    /// 客户端: 提供密码
    ///
    /// `password`: 密码
    GiveYouPassword { password: String },

    /// 服务器: 拒绝连接
    ///
    /// `reason`: 理由
    ConnectRefuse { reason: String },

    /// 服务器: 建立连接
    ///
    /// 这个连接不是 Tcp 意义上的连接, 请注意
    Welcome,

    /// 服务器: 确认, 无误
    Ok,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRequest {
    kind: MessageKind,
}
