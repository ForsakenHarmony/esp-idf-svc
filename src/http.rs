#[cfg(esp_idf_comp_esp_http_client_enabled)]
pub mod client;
#[cfg(esp_idf_comp_esp_http_server_enabled)]
pub mod server;

// use alloc::collections::BTreeMap;
// use core::ptr;
//
// use log::*;
//
// use esp_idf_sys::*;
//
// use crate::private::cstr::{self, CString};
// use embedded_svc::mutex::Mutex;
//
// enum Method {
//     Get,
//     Post,
//     Put,
//     Patch,
//     Delete,
//     Head,
//     Options,
// }
//
// impl Into<esp_http_client_method_t> for Method {
//     #[allow(non_upper_case_globals)]
//     fn into(self) -> esp_http_client_method_t {
//         match self {
//             Method::Get => esp_http_client_method_t_HTTP_METHOD_GET,
//             Method::Post => esp_http_client_method_t_HTTP_METHOD_POST,
//             Method::Put => esp_http_client_method_t_HTTP_METHOD_PUT,
//             Method::Patch => esp_http_client_method_t_HTTP_METHOD_PATCH,
//             Method::Delete => esp_http_client_method_t_HTTP_METHOD_DELETE,
//             Method::Head => esp_http_client_method_t_HTTP_METHOD_HEAD,
//             Method::Options => esp_http_client_method_t_HTTP_METHOD_OPTIONS,
//         }
//     }
// }
//
// enum HttpEvent {
//     /// This event occurs when there are any errors during execution
//     Error,
//     /// Once the HTTP has been connected to the server, no data exchange has been performed
//     Connected,
//     /// After sending all the headers to the server
//     HeadersSent,
//     ///  Occurs when receiving each header sent from the server
//     Header,
//     ///  Occurs when receiving data from the server, possibly multiple portions of the packet
//     Data,
//     /// Occurs when finish a HTTP session
//     Finish,
//     /// The connection has been disconnected
//     Disconnected,
// }
//
// impl From<esp_http_client_event_id_t> for HttpEvent {
//     #[allow(non_upper_case_globals)]
//     fn from(event_id: esp_http_client_event_id_t) -> Self {
//         match event_id {
//             esp_http_client_event_id_t_HTTP_EVENT_ERROR => HttpEvent::Error,
//             esp_http_client_event_id_t_HTTP_EVENT_ON_CONNECTED => HttpEvent::Connected,
//             esp_http_client_event_id_t_HTTP_EVENT_HEADERS_SENT => HttpEvent::HeadersSent,
//             esp_http_client_event_id_t_HTTP_EVENT_ON_HEADER => HttpEvent::Header,
//             esp_http_client_event_id_t_HTTP_EVENT_ON_DATA => HttpEvent::Data,
//             esp_http_client_event_id_t_HTTP_EVENT_ON_FINISH => HttpEvent::Finish,
//             esp_http_client_event_id_t_HTTP_EVENT_DISCONNECTED => HttpEvent::Disconnected,
//             _ => unreachable!("all http event ids are covered"),
//         }
//     }
// }
//
// const OUTPUT_BUFFER_LEN: usize = 2048;
//
// #[derive(Debug, Default)]
// pub struct Response {
//     headers: BTreeMap<String, String>,
//     body: Vec<u8>,
// }
//
// impl Response {
//     pub fn new() -> Self {
//         Default::default()
//     }
// }
//
// struct ClientHandle {
//     inner: esp_http_client_handle_t,
//     shared: Box<EspMutex<Option<Response>>>,
// }
//
// impl ClientHandle {
//     fn new(url: impl AsRef<str>) -> Result<Self, EspError> {
//         let mut shared = Box::new(EspMutex::new(None));
//         let shared_ref: *mut _ = &mut *shared;
//
//         let c_url = CString::new(url.as_ref()).unwrap();
//
//         let config = esp_http_client_config_t {
//             url: c_url.as_ptr(),
//             event_handler: Some(Self::http_event_handler),
//             user_data: shared_ref as _,
//             ..Default::default()
//         };
//
//         let handle = unsafe { esp_http_client_init(&config) };
//
//         if handle.is_null() {
//             return Err(EspError::from(ESP_ERR_INVALID_ARG as _).unwrap());
//         }
//
//         Ok(ClientHandle {
//             inner: handle,
//             shared,
//         })
//     }
//
//     extern "C" fn http_event_handler(evt: *mut esp_http_client_event_t) -> esp_err_t {
//         let evt = unsafe { &mut *evt };
//         let shared_ref =
//             unsafe { (evt.user_data as *mut mutex::EspMutex<Option<Response>>).as_mut() }.unwrap();
//
//         match HttpEvent::from(evt.event_id) {
//             HttpEvent::Error => {
//                 error!("http event: error");
//             }
//             HttpEvent::Connected => {
//                 info!("http event: connected");
//             }
//             HttpEvent::HeadersSent => {
//                 info!("http event: headers sent");
//             }
//             HttpEvent::Header => {
//                 let key = cstr::from_cstr_ptr(evt.header_key);
//                 let value = cstr::from_cstr_ptr(evt.header_value);
//                 info!("http event: header, key={}, value={}", key, value);
//                 shared_ref.with_lock(|r| {
//                     if let Some(r) = r.as_mut() {
//                         r.headers.insert(key, value);
//                     }
//                 })
//             }
//             HttpEvent::Data => {
//                 info!("http event: data");
//                 let chunked = unsafe { esp_http_client_is_chunked_response(evt.client) };
//                 if chunked {
//                     // TODO: handle chunked responses
//                     error!("chunked response in http event handler is not handled yet");
//                 } else {
//                     let len = unsafe { esp_http_client_get_content_length(evt.client) } as _;
//                     shared_ref.with_lock(|r| {
//                         if let Some(r) = r.as_mut() {
//                             r.body = Vec::with_capacity(len);
//                             unsafe {
//                                 ptr::copy(evt.data as _, r.body.as_mut_ptr(), len);
//                                 r.body.set_len(len);
//                             }
//                         }
//                     })
//                 }
//             }
//             HttpEvent::Finish => {
//                 info!("http event: finish");
//             }
//             HttpEvent::Disconnected => {
//                 info!("http event: disconnected");
//             }
//         }
//
//         return ESP_OK as _;
//     }
//
//     fn set_url(&mut self, url: impl AsRef<str>) -> Result<(), EspError> {
//         let c_url = CString::new(url.as_ref()).unwrap();
//
//         esp!(unsafe { esp_http_client_set_url(self.inner, c_url.as_ptr()) })
//     }
//
//     fn set_method(&mut self, method: Method) -> Result<(), EspError> {
//         esp!(unsafe { esp_http_client_set_method(self.inner, method.into()) })
//     }
//
//     fn set_header(&mut self, key: impl AsRef<str>, val: impl AsRef<str>) -> Result<(), EspError> {
//         let c_key = CString::new(key.as_ref()).unwrap();
//         let c_val = CString::new(val.as_ref()).unwrap();
//
//         esp!(unsafe { esp_http_client_set_header(self.inner, c_key.as_ptr(), c_val.as_ptr()) })
//     }
//
//     fn set_body(&mut self, body: Option<&[u8]>) -> Result<(), EspError> {
//         if let Some(body) = body {
//             esp!(unsafe {
//                 esp_http_client_set_post_field(self.inner, body.as_ptr() as _, body.len() as _)
//             })
//         } else {
//             esp!(unsafe { esp_http_client_set_post_field(self.inner, ptr::null(), 0) }).or_else(
//                 |e| {
//                     if e.code() == ESP_ERR_NOT_FOUND as c_types::c_int {
//                         Ok(())
//                     } else {
//                         Err(e)
//                     }
//                 },
//             )
//         }
//     }
//
//     fn perform(&mut self) -> Result<Response, EspError> {
//         self.shared.with_lock(|s| {
//             s.insert(Response::new());
//         });
//
//         esp!(unsafe { esp_http_client_perform(self.inner) })?;
//
//         let res = self.shared.with_lock(|s| s.take().unwrap());
//
//         self.set_body(None)?;
//
//         Ok(res)
//     }
// }
//
// impl Drop for ClientHandle {
//     fn drop(&mut self) {
//         let _ = unsafe { esp_http_client_cleanup(self.inner) };
//     }
// }
//
// #[derive(Default)]
// pub struct Client {
//     client_handle: Option<ClientHandle>,
// }
//
// pub struct Request {
// 	client: Client,
// }
//
// impl Request {
// 	pub fn new() -> Self {}
// }
//
// impl Client {
//     pub fn new() -> Self {
//         Default::default()
//     }
//
//     fn handle(&mut self, url: impl AsRef<str>) -> Result<&mut ClientHandle, EspError> {
//         if let Some(handle) = self.client_handle.as_mut() {
//             handle.set_url(url)?;
//         } else {
//             let handle = ClientHandle::new(url)?;
//             self.client_handle = Some(handle);
//         }
//
//         Ok(self.client_handle.as_mut().unwrap())
//     }
//
//     fn request(
//         &mut self,
//         url: impl AsRef<str>,
//         method: Method,
//         // headers: Option<impl Iterator<Item = (dyn AsRef<str>, dyn AsRef<str>)>>,
//         body: Option<&[u8]>,
//     ) -> Result<Request, EspError> {
//         let handle = self.handle(url)?;
//
//         handle.set_method(method)?;
//         // for (k, v) in headers.into_iter().flatten() {
//         //     handle.set_header(k, v)?;
//         // }
//         handle.set_body(body)?;
//
//         let res = handle.perform()?;
//
//         Ok(res)
//     }
//
//     pub fn get(&mut self, url: impl AsRef<str>) -> Result<Request, EspError> {
//         info!("GET {}", url.as_ref());
//         self.request(url, Method::Get, None)
//     }
//
//     // FIXME: set content type
//     pub fn post(&mut self, url: impl AsRef<str>, body: &[u8]) -> Result<Request, EspError> {
//         self.request(url, Method::Post, Some(body))
//     }
//
//     pub fn patch(&mut self, url: impl AsRef<str>, body: &[u8]) -> Result<Request, EspError> {
//         self.request(url, Method::Patch, Some(body))
//     }
//
//     pub fn put(&mut self, url: impl AsRef<str>, body: &[u8]) -> Result<Request, EspError> {
//         self.request(url, Method::Put, Some(body))
//     }
//
//     pub fn delete(&mut self, url: impl AsRef<str>, body: &[u8]) -> Result<Request, EspError> {
//         self.request(url, Method::Delete, Some(body))
//     }
// }
