use itertools::Itertools;
use pallas_codec::minicbor::{decode, encode, Decode, Decoder, Encode, Encoder};
use std::{collections::HashMap, fmt::Debug};

#[derive(Debug, Clone)]
pub struct VersionTable<T>
where
    T: Debug + Clone,
{
    pub values: HashMap<u64, T>,
}

impl<T> Encode<()> for VersionTable<T>
where
    T: Debug + Clone + Encode<()>,
{
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        e.map(self.values.len() as u64)?;

        for key in self.values.keys().sorted() {
            e.u64(*key)?;
            e.encode(&self.values[key])?;
        }

        Ok(())
    }
}

impl<'b, T> Decode<'b, ()> for VersionTable<T>
where
    T: Debug + Clone + Decode<'b, ()>,
{
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let len = d.map()?.ok_or(decode::Error::message(
            "expected def-length map for versiontable",
        ))?;
        let mut values = HashMap::new();

        for _ in 0..len {
            let key = d.u64()?;
            let value = d.decode()?;
            values.insert(key, value);
        }
        Ok(VersionTable { values })
    }
}

pub type NetworkMagic = u64;

pub type VersionNumber = u64;

#[derive(Debug)]
pub enum Message<D>
where
    D: Debug + Clone,
{
    Propose(VersionTable<D>),
    Accept(VersionNumber, D),
    Refuse(RefuseReason),
    QueryReply(VersionTable<D>),
}

impl<D> Encode<()> for Message<D>
where
    D: Debug + Clone,
    D: Encode<()>,
    VersionTable<D>: Encode<()>,
{
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            Message::Propose(version_table) => {
                e.array(2)?.u16(0)?;
                e.encode(version_table)?;
            }
            Message::Accept(version_number, version_data) => {
                e.array(3)?.u16(1)?;
                e.u64(*version_number)?;
                e.encode(version_data)?;
            }
            Message::Refuse(reason) => {
                e.array(2)?.u16(2)?;
                e.encode(reason)?;
            }
            Message::QueryReply(version_table) => {
                e.array(2)?.u16(3)?;
                e.encode(version_table)?;
            }
        };

        Ok(())
    }
}

impl<'b, D> Decode<'b, ()> for Message<D>
where
    D: Decode<'b, ()> + Debug + Clone,
    VersionTable<D>: Decode<'b, ()>,
{
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;

        match d.u16()? {
            0 => {
                let version_table = d.decode()?;
                Ok(Message::Propose(version_table))
            }
            1 => {
                let version_number = d.u64()?;
                let version_data = d.decode()?;
                Ok(Message::Accept(version_number, version_data))
            }
            2 => {
                let reason: RefuseReason = d.decode()?;
                Ok(Message::Refuse(reason))
            }
            3 => {
                let version_table = d.decode()?;
                Ok(Message::QueryReply(version_table))
            }
            _ => Err(decode::Error::message(
                "unknown variant for handshake message",
            )),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Propose,
    Confirm,
    Done,
}

#[derive(Debug)]
pub enum RefuseReason {
    VersionMismatch(Vec<VersionNumber>),
    HandshakeDecodeError(VersionNumber, String),
    Refused(VersionNumber, String),
}

impl Encode<()> for RefuseReason {
    fn encode<W: encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut (),
    ) -> Result<(), encode::Error<W::Error>> {
        match self {
            RefuseReason::VersionMismatch(versions) => {
                e.array(2)?;
                e.u16(0)?;
                e.array(versions.len() as u64)?;
                for v in versions.iter() {
                    e.u64(*v)?;
                }

                Ok(())
            }
            RefuseReason::HandshakeDecodeError(version, msg) => {
                e.array(3)?;
                e.u16(1)?;
                e.u64(*version)?;
                e.str(msg)?;

                Ok(())
            }
            RefuseReason::Refused(version, msg) => {
                e.array(3)?;
                e.u16(2)?;
                e.u64(*version)?;
                e.str(msg)?;

                Ok(())
            }
        }
    }
}

impl<'b> Decode<'b, ()> for RefuseReason {
    fn decode(d: &mut Decoder<'b>, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;

        match d.u16()? {
            0 => {
                let versions = d.array_iter::<u64>()?;
                let versions = versions.try_collect()?;
                Ok(RefuseReason::VersionMismatch(versions))
            }
            1 => {
                let version = d.u64()?;
                let msg = d.str()?;

                Ok(RefuseReason::HandshakeDecodeError(version, msg.to_string()))
            }
            2 => {
                let version = d.u64()?;
                let msg = d.str()?;

                Ok(RefuseReason::Refused(version, msg.to_string()))
            }
            _ => Err(decode::Error::message("unknown variant for refusereason")),
        }
    }
}
