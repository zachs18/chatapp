use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message<'a> {
    NameAssignment(Cow<'a, str>),

    ChatMessage(Cow<'a, str>),
    ChatMessageError(u8),

    NameChangeRequest(Cow<'a, str>),
    NameChangeApproval,
    NameChangeDenial(u8),

    Disconnect,
}

impl<'a> Message<'a> {
    pub fn message_type(&self) -> u8 {
        use Message::*;
        match self {
            NameAssignment(_) => 0,
            ChatMessage(_) => 64,
            ChatMessageError(_) => 65,
            NameChangeRequest(_) => 128,
            NameChangeApproval => 129,
            NameChangeDenial(_) => 130,
            Disconnect => 255,
        }
    }

    pub fn from_bytes(msg: &'a [u8]) -> Option<Self> {
        use Message::*;
        if msg.len() < 1 { return None; }
        Some(match msg.split_at(1) {
            (&[0], name) => NameAssignment(std::str::from_utf8(name).ok()?.into()),
            (&[64], message) => ChatMessage(std::str::from_utf8(message).ok()?.into()),
            (&[65], &[error]) => ChatMessageError(error),
            (&[128], name) => NameChangeRequest(std::str::from_utf8(name).ok()?.into()),
            (&[129], &[]) => NameChangeApproval,
            (&[130], &[error]) => NameChangeDenial(error),
            (&[255], &[]) => Disconnect,
            _ => return None,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        use Message::*;
        let mut bytes = vec![self.message_type()];
        match self {
            NameAssignment(name) => {
                bytes.reserve(name.len());
                bytes.extend(name.as_bytes());
            },
            ChatMessage(message) => {
                bytes.reserve(message.len());
                bytes.extend(message.as_bytes());
            },
            ChatMessageError(error) => {
                bytes.push(*error);
            },
            NameChangeRequest(name) => {
                bytes.reserve(name.len());
                bytes.extend(name.as_bytes());
            },
            NameChangeApproval => {},
            NameChangeDenial(error) => {
                bytes.push(*error);
            },
            Disconnect => {},
        };
        bytes
    }
}
