use super::ForwardedResolver;
use crate::store::SharedRoutes;
use async_trait::async_trait;
use hickory_client::client::{Client, ClientHandle};
use hickory_proto::op::{Message as DnsMessage, MessageType, OpCode, ResponseCode};
use hickory_proto::rr::record_type::RecordType as ClientRecordType;
use hickory_proto::rr::{Name, RData, Record, RecordType};
use hickory_proto::udp::UdpClientStream;
use hickory_server::authority::MessageResponseBuilder;
use hickory_server::server::{Request, RequestHandler, ResponseHandler, ResponseInfo};
use std::net::IpAddr;

pub struct CustomHandler {
    pub routes: SharedRoutes,
    pub upstream_addr: ForwardedResolver,
}

#[async_trait]
impl RequestHandler for CustomHandler {
    async fn handle_request<R: ResponseHandler>(&self, request: &Request, mut response_handle: R) -> ResponseInfo {
        let response = MessageResponseBuilder::from_message_request(request);
        let mut message = DnsMessage::new();
        message.set_id(request.id());
        message.set_message_type(MessageType::Response);
        message.set_op_code(OpCode::Query);
        if request.recursion_desired() {
            message.set_recursion_available(true);
            message.set_recursion_desired(true);
        }

        for q in request.queries() {
            let qname_fqdn = q.name().to_utf8();
            let qname = qname_fqdn.trim_end_matches('.').to_string();
            let qtype = q.query_type();
            let matched = {
                let map = self.routes.read();
                map.iter().any(|r| r.domain.trim_end_matches('.') == qname)
            };

            if matched {
                let ip = {
                    let map = self.routes.read();
                    map.iter()
                        .find(|r| r.domain.trim_end_matches('.') == qname)
                        .map(|r| r.ip)
                        .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)))
                };
                tracing::trace!(domain = %qname, qtype = ?qtype, ip = %ip, "dns: intercepted");
                if qtype == RecordType::A {
                    let mut rec = Record::update0(
                        Name::from_utf8(&qname_fqdn).unwrap_or_else(|_| q.name().to_lowercase()),
                        300,
                        RecordType::A,
                    );
                    if let IpAddr::V4(v4) = ip {
                        rec.set_data(RData::A(hickory_proto::rr::rdata::A(v4)));
                    }
                    message.add_answer(rec);
                    message.set_response_code(ResponseCode::NoError);
                } else if qtype == RecordType::AAAA {
                    if let IpAddr::V6(v6) = ip {
                        let mut rec = Record::update0(
                            Name::from_utf8(&qname_fqdn).unwrap_or_else(|_| q.name().to_lowercase()),
                            300,
                            RecordType::AAAA,
                        );
                        rec.set_data(RData::AAAA(hickory_proto::rr::rdata::AAAA(v6)));
                        message.add_answer(rec);
                        message.set_response_code(ResponseCode::NoError);
                    } else {
                        message.set_response_code(ResponseCode::NXDomain);
                    }
                } else {
                    message.set_response_code(ResponseCode::NXDomain);
                }
                continue;
            } else if let Some(mut resp) = forward_upstream(q.name().to_lowercase(), qtype, &self.upstream_addr).await {
                tracing::trace!(domain = %qname, qtype = ?qtype, "dns: forwarded upstream");
                resp.set_id(request.id());
                message = resp;
                break;
            } else {
                tracing::debug!(domain = %qname, qtype = ?qtype, "dns: no match, upstream unavailable");
                message.set_response_code(ResponseCode::ServFail);
            }
        }

        response_handle
            .send_response(response.build(
                *message.header(),
                message.answers(),
                message.name_servers(),
                message.name_servers(),
                message.additionals(),
            ))
            .await
            .unwrap()
    }
}

async fn forward_upstream(name: Name, qtype: RecordType, upstream_addr: &ForwardedResolver) -> Option<DnsMessage> {
    #[allow(clippy::clone_on_copy)]
    let upstream = upstream_addr.read().clone()?;
    let stream = UdpClientStream::builder(upstream, hickory_proto::runtime::TokioRuntimeProvider::default()).build();
    if let Ok((mut client, bg)) = Client::connect(stream).await {
        tokio::spawn(bg);
        let qclass = hickory_proto::rr::DNSClass::IN;
        let qtype_client = match qtype {
            RecordType::A => ClientRecordType::A,
            RecordType::AAAA => ClientRecordType::AAAA,
            _ => ClientRecordType::A,
        };
        if let Ok(resp) = client.query(name, qclass, qtype_client).await {
            return Some(resp.into_message());
        }
    }
    None
}
