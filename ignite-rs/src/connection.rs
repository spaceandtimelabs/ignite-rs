use std::io::{Read, Write};
use std::net::TcpStream;

use crate::api::OpCode;
use crate::error::{Error, Result};
use crate::handshake::handshake;
use crate::protocol::Flag::{Failure, Success};
use crate::protocol::{read_i32, read_i64, write_i16, write_i32, write_i64, Flag};
use crate::{ClientConfig, ReadableReq};
use crate::{ReadableType, WriteableReq};
use bufstream::BufStream;
#[cfg(feature = "ssl")]
use rustls;
use std::io;
use std::option::Option::Some;
#[allow(unused_imports)]
use std::sync::{Arc, Mutex};
#[cfg(feature = "ssl")]
use webpki;

const DFLT_READ_BUF_SIZE: usize = 1024;
const DFLT_WRITE_BUF_SIZE: usize = 1024;
const REQ_HEADER_SIZE_BYTES: i32 = 10;

pub struct Connection {
    #[cfg(not(feature = "ssl"))]
    stream: Mutex<BufStream<TcpStream>>,
    #[cfg(feature = "ssl")]
    stream: Mutex<BufStream<rustls::StreamOwned<rustls::ClientSession, TcpStream>>>,
}

impl Connection {
    pub(crate) fn new(conf: &ClientConfig) -> Result<Connection> {
        match TcpStream::connect(&conf.addr) {
            Ok(stream) => {
                // apply tcp configs
                Connection::configure_tcp(&stream, conf)?;

                // wrap in tls stream if this feature enabled
                #[cfg(feature = "ssl")]
                let mut stream = Connection::wrap_tls_stream(&conf.tls_conf, stream)?;

                // wrap in buffered stream
                let mut buffered_stream = BufStream::with_capacities(
                    conf.tcp_read_buff_size.unwrap_or(DFLT_READ_BUF_SIZE),
                    conf.tcp_write_buff_size.unwrap_or(DFLT_WRITE_BUF_SIZE),
                    stream,
                );

                // try initial handshake
                match handshake(&mut buffered_stream, conf) {
                    Ok(_) => Ok(Connection {
                        stream: Mutex::new(buffered_stream),
                    }),
                    Err(err) => Err(err),
                }
            }
            Err(err) => Err(Error::from(err)),
        }
    }

    /// Send message and read response header. Acquires lock
    pub(crate) fn send(&self, op_code: OpCode, data: impl WriteableReq) -> Result<()> {
        let sock_lock = &mut *self.stream.lock()?; //acquire lock on socket
        Connection::send_safe(sock_lock, op_code, data)
    }

    /// Send message, read response header and return a response. Acquires lock
    pub(crate) fn send_and_read<T: ReadableReq>(
        &self,
        op_code: OpCode,
        data: impl WriteableReq,
    ) -> Result<T> {
        let sock_lock = &mut *self.stream.lock()?; //acquire lock on socket
        Connection::send_and_read_safe(sock_lock, op_code, data)
    }

    /// Send message, let the caller read the result. Acquires lock
    pub(crate) fn send_and_read_dyn(
        &self,
        op_code: OpCode,
        req: impl WriteableReq,
        cb: &mut dyn Fn(&mut dyn Read) -> Result<()>,
    ) -> Result<()> {
        let buf = &mut *self.stream.lock()?; //acquire lock on socket
        Connection::send_safe(buf, op_code, req)?; //send request and read the response
        cb(&mut Box::new(buf))?;
        Ok(())
    }

    fn send_safe<RW: Read + Write>(
        con: &mut RW,
        op_code: OpCode,
        payload: impl WriteableReq,
    ) -> Result<()> {
        // write common message header
        Connection::write_req_header(con, payload.size(), op_code as i16)?;

        // write payload
        payload.write(con)?;

        // flush write buffer
        con.flush()?;

        //read response
        match Connection::read_resp_header(con)? {
            Flag::Success => Ok(()),
            Flag::Failure { err_msg } => Err(Error::from(err_msg.as_str())),
        }
    }

    fn send_and_read_safe<T: ReadableReq, RW: Read + Write>(
        buf: &mut RW,
        op_code: OpCode,
        data: impl WriteableReq,
    ) -> Result<T> {
        Connection::send_safe(buf, op_code, data)?; //send request and read the response
        T::read(buf) //unpack the input bytes into an actual type
    }

    /// Returns binary repr of standard request header
    fn write_req_header(
        writer: &mut dyn Write,
        payload_len: usize,
        op_code: i16,
    ) -> io::Result<()> {
        write_i32(writer, payload_len as i32 + REQ_HEADER_SIZE_BYTES)?;
        write_i16(writer, op_code)?;
        write_i64(writer, 0)?; // Request id
        Ok(())
    }

    /// Reads standard response header
    fn read_resp_header(reader: &mut impl Read) -> Result<Flag> {
        let _length = read_i32(reader)?;
        let _req_id = read_i64(reader)?;
        let status = read_i32(reader)?;
        match status {
            0 => Ok(Success),
            _ => {
                let err_msg = String::read(reader)?;
                Ok(Failure {
                    err_msg: err_msg.unwrap(),
                })
            }
        }
    }

    #[cfg(feature = "ssl")]
    fn wrap_tls_stream(
        conf: &(rustls::ClientConfig, String),
        stream: TcpStream,
    ) -> Result<rustls::StreamOwned<rustls::ClientSession, TcpStream>> {
        let hostname = webpki::DNSNameRef::try_from_ascii_str(&conf.1)?;
        let tls_session = rustls::ClientSession::new(&Arc::new(conf.0.clone()), hostname);
        let tls_stream = rustls::StreamOwned::new(tls_session, stream);
        Ok(tls_stream)
    }

    fn configure_tcp(stream: &TcpStream, conf: &ClientConfig) -> io::Result<()> {
        stream.set_read_timeout(conf.tcp_read_timeout)?;
        stream.set_write_timeout(conf.tcp_write_timeout)?;
        if let Some(nodelay) = conf.tcp_nodelay {
            stream.set_nodelay(nodelay)?;
        }
        if let Some(nonblocking) = conf.tcp_nonblocking {
            stream.set_nonblocking(nonblocking)?;
        }
        if let Some(ttl) = conf.tcp_ttl {
            stream.set_ttl(ttl)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::complex_obj::{ComplexObject, ComplexObjectSchema, IgniteValue};
    use crate::{new_client, Ignite};

    #[ignore]
    #[test]
    fn test_read() {
        let config = ClientConfig::new("localhost:10800");
        let mut ignite = new_client(config).unwrap();
        let table_name = "SQL_PUBLIC_BLOCKS";
        let cfg = ignite.get_cache_config(table_name).unwrap();
        let entity = cfg.query_entities.unwrap().last().unwrap().clone();
        println!("type_name={}", entity.value_type);
        let cache = ignite
            .get_or_create_cache::<i64, ComplexObject>(table_name)
            .unwrap();
        let rows = cache.query_scan(100).unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[ignore]
    #[test]
    fn test_crud() {
        let config = ClientConfig::new("localhost:10800");
        let mut ignite = new_client(config).unwrap();
        let table_name = "SQL_PUBLIC_BOOL_TEST";
        // println!("cache names: {:?}", ignite.get_cache_names());
        let cfg = ignite.get_cache_config(table_name).unwrap();
        let entity = cfg.query_entities.unwrap().last().unwrap().clone();
        let (ks, vs) = ComplexObjectSchema::infer_schemas(&entity).unwrap();
        println!("ks={ks:?} vs={vs:?}");
        let _type_name = entity.value_type.split(".").last().unwrap();
        println!("value_type={}", entity.value_type);
        let key = ComplexObject {
            schema: ks.clone(),
            values: vec![IgniteValue::Long(1)],
        };
        let val = ComplexObject {
            schema: vs.clone(),
            values: vec![IgniteValue::Bool(true)],
        };

        // let tx_id = ignite.start_transaction().unwrap();
        let cache = ignite
            .get_or_create_cache::<ComplexObject, ComplexObject>(table_name)
            .unwrap();
        cache.put(&key, &val).unwrap();
        // ignite.end_transaction(tx_id, false).unwrap();

        let rows = cache.query_scan(100).unwrap();
        // let rows = cache
        //     .query_scan_sql(100, type_name, "order by block_number desc limit 1")
        //     .unwrap();
        assert_eq!(rows.len(), 1);
    }
}
