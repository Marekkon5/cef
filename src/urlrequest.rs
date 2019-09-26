use cef_sys::{cef_urlrequest_t, cef_urlrequest_client_t, cef_auth_callback_t, cef_urlrequest_status_t, cef_base_ref_counted_t, cef_response_t, cef_request_context_t, cef_string_t, cef_response_filter_status_t, cef_request_callback_t};
use num_enum::UnsafeFromPrimitive;

use crate::{
    request::Request,
    load_handler::ErrorCode,
    string::CefString,
    refcounted::{RefCounted, RefCounter},
    browser::Browser,
    frame::Frame,
    web_plugin::WebPluginInfo,
    ReturnValue,
    cookie::Cookie,
    callback::Callback,
    resource_request::{ResourceRequestHandler},
    request_context::RequestContext,
};

/// Flags that represent [URLRequest] status.
#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq, UnsafeFromPrimitive)]
pub enum URLRequestStatus {
    /// Unknown status.
    Unknown = cef_urlrequest_status_t::UR_UNKNOWN as i32,
    /// Request succeeded.
    Success = cef_urlrequest_status_t::UR_SUCCESS as i32,
    /// An IO request is pending, and the caller will be informed when it is
    /// completed.
    IOPending = cef_urlrequest_status_t::UR_IO_PENDING as i32,
    /// Request was canceled programatically.
    Canceled = cef_urlrequest_status_t::UR_CANCELED as i32,
    /// Request failed for some reason.
    Failed = cef_urlrequest_status_t::UR_FAILED as i32,
}

/// Structure used to make a URL request. URL requests are not associated with a
/// browser instance so no [Client] callbacks will be executed. URL requests
/// can be created on any valid CEF thread in either the browser or render
/// process. Once created the functions of the URL request object must be
/// accessed on the same thread that created it.
pub struct URLRequest(*mut cef_urlrequest_t);

impl URLRequest {
    /// Create a new URL request that is not associated with a specific browser or
    /// frame. Use [Frame::create_urlrequest] instead if you want the request to
    /// have this association, in which case it may be handled differently (see
    /// documentation on that function). Requests may originate from the both browser
    /// process and the render process.
    ///
    /// For requests originating from the browser process:
    ///   - It may be intercepted by the client via [ResourceRequestHandler] or
    ///     [SchemeHandlerFactory].
    ///   - POST data may only contain only a single element of type [PostDataElementType::File]
    ///     or [PostDataElementType::Bytes].
    ///   - If `request_context` is empty the global request context will be used.
    /// For requests originating from the render process:
    ///   - It cannot be intercepted by the client so only http(s) and blob schemes
    ///     are supported.
    ///   - POST data may only contain a single element of type [PostDataElementType::Bytes].
    ///   - The `request_context` parameter must be None.
    ///
    /// The `request` object will be marked as read-only after calling this function.
    pub fn new(request: &mut Request, client: Box<dyn URLRequestClient>, request_contest: Option<&RequestContext>) -> Self {
        //cef_urlrequest_create
        unimplemented!()
    }
    /// Returns the request object used to create this URL request. The returned
    /// object is read-only and should not be modified.
    pub fn get_request(&self) -> Request {
        unimplemented!()
    }
    /// Returns the client.
    pub fn get_client(&self) -> impl URLRequestClient {
        unimplemented!()
    }
    /// Returns the request status.
    pub fn get_request_status(&self) -> URLRequestStatus {
        unimplemented!()
    }
    /// Returns the request error if status is [URLRequestStatus::Canceled] or [URLRequestStatus::Failed], or [ErrorCode::None]
    /// otherwise.
    pub fn get_request_error(&self) -> ErrorCode {
        unimplemented!()
    }
    /// Returns the response, or None if no response information is available.
    /// Response information will only be available after the upload has completed.
    /// The returned object is read-only and should not be modified.
    pub fn get_response(&self) -> Option<Response> {
        unimplemented!()
    }
    /// Returns true if the response body was served from the cache. This
    /// includes responses for which revalidation was required.
    pub fn response_was_cached(&self) -> bool {
        unimplemented!()
    }
    /// Cancel the request.
    pub fn cancel(&self) {
        unimplemented!()
    }
}

impl From<*mut cef_urlrequest_t> for URLRequest {
    fn from(req: *mut cef_urlrequest_t) -> Self {
        unsafe {
            ((*req).base.add_ref.unwrap())(&mut (*req).base);
        }
        Self(req)
    }
}

impl Drop for URLRequest {
    fn drop(&mut self) {
        unsafe {
            ((&*self.0).base.release.unwrap())(&mut (&mut *self.0).base);
        }
    }
}

/// Callback structure used for asynchronous continuation of authentication
/// requests.
pub struct AuthCallback(*mut cef_auth_callback_t);

unsafe impl Send for AuthCallback {}
unsafe impl Sync for AuthCallback {}

impl AuthCallback {
    /// Continue the authentication request.
    pub fn cont(&self, username: &str, password: &str) {
        if let Some(cont) = unsafe { &*self.0 }.cont {
            unsafe { cont(self.0, CefString::new(username).as_ref(), CefString::new(password).as_ref()); }
        }
        ((&*self.0).base.release.unwrap())(&mut (&mut *self.0).base);
    }
    /// Cancel the authentication request.
    pub fn cancel(&self) {
        if let Some(cancel) = unsafe { &*self.0 }.cancel {
            unsafe { cancel(self.0); }
        }
        ((&*self.0).base.release.unwrap())(&mut (&mut *self.0).base);
    }
}

impl Drop for AuthCallback {
    fn drop(&mut self) {
        unsafe {
            ((&*self.0).base.release.unwrap())(&mut (&mut *self.0).base);
        }
    }
}

impl From<*mut cef_auth_callback_t> for AuthCallback {
    fn from(cb: *mut cef_auth_callback_t) -> Self {
        unsafe {
            ((*cb).base.add_ref.unwrap())(&mut (*cb).base);
        }
        Self(cb)
    }
}

/// Trait that should be implemented by the [URLRequest] client. The
/// functions of this trait will be called on the same thread that created
/// the request unless otherwise documented.
pub trait URLRequestClient: Send + Sync {
    /// Notifies the client that the request has completed. Use the
    /// [URLRequest::get_request_status] function to determine if the request was
    /// successful or not.
    fn on_request_complete(&self, request: &URLRequest) {}
    /// Notifies the client of upload progress. `current` denotes the number of
    /// bytes sent so far and `total` is the total size of uploading data (or -1 if
    /// chunked upload is enabled). This function will only be called if the
    /// [URLRequestFlags::ReportUploadProgress] flag is set on the request.
    fn on_upload_progress(&self, request: &URLRequest, current: i64, total: i64) {}
    /// Notifies the client of download progress. `current` denotes the number of
    /// bytes received up to the call and `total` is the expected total size of the
    /// response (or -1 if not determined).
    fn on_download_progress(&self, request: &URLRequest, current: i64, total: i64) {}
    /// Called when some part of the response is read. `data` contains the current
    /// bytes received since the last call. This function will not be called if the
    /// [URLRequestFlags::NoDownloadData] flag is set on the request.
    fn on_download_data(&self, request: &URLRequest, data: &[u8]) {}
    /// Called on the IO thread when the browser needs credentials from the user.
    /// `is_proxy` indicates whether the host is a proxy server. `host` contains the
    /// hostname and `port` contains the port number. Return true to continue
    /// the request and call [AuthCallback::cont] when the authentication
    /// information is available. If the request has an associated browser/frame
    /// then returning false will result in a call to [RequestHandler::GetAuthCredentials] on the
    /// [RequestHandler] associated with that browser, if any. Otherwise,
    /// returning false will cancel the request immediately. This function will
    /// only be called for requests initiated from the browser process.
    fn get_auth_credentials(&self, is_proxy: bool, host: &str, port: u16, realm: &str, scheme: &str, callback: AuthCallback) -> bool { false }
}

impl RefCounter for cef_urlrequest_client_t {
    type Wrapper = RefCounted<Self, Box<dyn URLRequestClient>>;
    fn set_base(&mut self, base: cef_base_ref_counted_t) {
        self.base = base;
    }
}

pub(crate) struct URLRequestClientWrapper();

impl URLRequestClientWrapper {
    pub(crate) fn wrap(client: Box<dyn URLRequestClient>) -> *mut <cef_urlrequest_client_t as RefCounter>::Wrapper {
        RefCounted::new(cef_urlrequest_client_t {
            on_request_complete: Some(Self::request_complete),
            on_upload_progress: Some(Self::upload_progress),
            on_download_progress: Some(Self::download_progress),
            on_download_data: Some(Self::download_data),
            get_auth_credentials: Some(Self::get_auth_credentials),
            ..Default::default()
        }, client)
    }

    extern "C" fn request_complete(self_: *mut cef_urlrequest_client_t, request: *mut cef_urlrequest_t) {
        let mut this = unsafe { <cef_urlrequest_client_t as RefCounter>::Wrapper::make_temp(self_) };
        (*this).on_request_complete(&URLRequest::from(request));
    }
    extern "C" fn upload_progress(self_: *mut cef_urlrequest_client_t, request: *mut cef_urlrequest_t, current: i64, total: i64) {
        let mut this = unsafe { <cef_urlrequest_client_t as RefCounter>::Wrapper::make_temp(self_) };
        (*this).on_upload_progress(&URLRequest::from(request), current, total);
    }
    extern "C" fn download_progress(self_: *mut cef_urlrequest_client_t, request: *mut cef_urlrequest_t, current: i64, total: i64) {
        let mut this = unsafe { <cef_urlrequest_client_t as RefCounter>::Wrapper::make_temp(self_) };
        (*this).on_download_progress(&URLRequest::from(request), current, total);
    }
    extern "C" fn download_data(self_: *mut cef_urlrequest_client_t, request: *mut cef_urlrequest_t, data: *const std::os::raw::c_void, data_length: usize) {
        let mut this = unsafe { <cef_urlrequest_client_t as RefCounter>::Wrapper::make_temp(self_) };
        (*this).on_download_data(&URLRequest::from(request), unsafe { std::slice::from_raw_parts(data as *const u8, data_length) });
    }
    extern "C" fn get_auth_credentials(self_: *mut cef_urlrequest_client_t, is_proxy: std::os::raw::c_int, host: *const cef_string_t, port: std::os::raw::c_int, realm: *const cef_string_t, scheme: *const cef_string_t, callback: *mut cef_auth_callback_t) -> i32 {
        let mut this = unsafe { <cef_urlrequest_client_t as RefCounter>::Wrapper::make_temp(self_) };
        (*this).get_auth_credentials(is_proxy != 0, &CefString::copy_raw_to_string(host).unwrap(), port as u16, &CefString::copy_raw_to_string(realm).unwrap(), &CefString::copy_raw_to_string(scheme).unwrap(), AuthCallback::from(callback)) as i32
    }
}

/// Structure used to represent a web response. The functions of this structure
/// may be called on any thread.
pub struct Response(*mut cef_response_t);

unsafe impl Send for Response {}
unsafe impl Sync for Response {}

pub struct RequestCallback(*mut cef_request_callback_t);

/// Implement this trait to filter cookies that may be sent or received from
/// resource requests. The functions of this trait will be called on the IO
/// thread unless otherwise indicated.
pub trait CookieAccessFilter: Sync + Send {
    /// Called on the IO thread before a resource request is sent. The `browser`
    /// and `frame` values represent the source of the request, and may be None for
    /// requests originating from service workers or [URLRequest].
    /// Return true if the specified  cookie can be sent with the request or false otherwise.
    fn can_send_cookie(&self, browser: Option<&Browser>, frame: Option<&Frame>, request: &Request, cookie: &Cookie) -> bool { false }
    /// Called on the IO thread after a resource response is received. The
    /// `browser` and `frame` values represent the source of the request, and may
    /// be None for requests originating from service workers or [URLRequest].
    /// Return true if the
    /// specified cookie returned with the response can be saved or false
    /// otherwise.
    fn can_save_cookie(&self, browser: Option<&Browser>, frame: Option<&Frame>, request: &Request, response: &Response, cookie: &Cookie) -> bool { false }
}

#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, UnsafeFromPrimitive)]
pub enum ResponseFilterStatus {
    NeedMoreData = cef_response_filter_status_t::RESPONSE_FILTER_NEED_MORE_DATA as i32,
    Done         = cef_response_filter_status_t::RESPONSE_FILTER_DONE as i32,
    Error        = cef_response_filter_status_t::RESPONSE_FILTER_ERROR as i32,
}

/// Implement this trait to filter resource response content. The functions
/// of this trait will be called on the browser process IO thread.
pub trait ResponseFilter: Send + Sync {
    /// Initialize the response filter. Will only be called a single time. The
    /// filter will not be installed if this function returns false.
    fn init_filter(&self) -> bool { false }
    /// Called to filter a chunk of data. Expected usage is as follows:
    ///
    ///  A. Read input data from `data_in` and set `data_in_read` to the number of
    ///     bytes that were read up to a maximum of `data_in.len()`. `data_in` can
    ///     be size 0.
    ///  B. Write filtered output data to `data_out` and set `data_out_written` to
    ///     the number of bytes that were written up to a maximum of
    ///     `data_out.len()`. If no output data was written then all data must be
    ///     read from `data_in` (user must set `data_in_read` = `data_in.len()`).
    ///  C. Return [ResponseFilterStatus::Done] if all output data was written or
    ///     [ResponseFilterStatus::NeedMoreData] if output data is still pending.
    ///
    /// This function will be called repeatedly until the input slice has been
    /// fully read (user sets `data_in_read` = `data_in.len()`) and there is no more
    /// input data to filter (the resource response is complete). This function may
    /// then be called an additional time with an zero-length input slice if the user
    /// filled the output slice (set `data_out_written` = `data_out.len()`) and
    /// returned [ResponseFilterStatus::NeedMoreData] to indicate that output data is
    /// still pending.
    ///
    /// Calls to this function will stop when one of the following conditions is
    /// met:
    ///
    ///  A. There is no more input data to filter (the resource response is
    ///     complete) and the user sets `data_out_written` = 0 or returns
    ///     [ResponseFilterStatus::Done] to indicate that all data has been written, or;
    ///  B. The user returns [ResponseFilterStatus::Error] to indicate an error.
    fn filter(&self, data_in: &[u8], data_in_read: &mut usize, data_out: &[u8], data_out_written: &mut usize) -> ResponseFilterStatus { ResponseFilterStatus::Error }
}

/// Structure used to implement a custom request handler structure. The functions
/// of this structure will be called on the IO thread unless otherwise indicated.
pub trait ResourceHandler: Send + Sync {
    /// Open the response stream. To handle the request immediately set
    /// `handle_request` to true and return true. To decide at a later time
    /// set `handle_request` to false, return true, and execute `callback`
    /// to continue or cancel the request. To cancel the request immediately set
    /// `handle_request` to true and return false. This function will be
    /// called in sequence but not from a dedicated thread.
    fn open(&self, request: &Request, handle_request: &bool, callback: Callback) -> bool {
        (*handle_request) = true;
        false
    }
    /// Retrieve response header information. If the response length is not known
    /// set `response_length` to -1 and [ResourceHandler::read_response] will be called until it
    /// returns false. If the response length is known set `response_length` to
    /// a positive value and [ResourceHandler::read_response] will be called until it returns false
    /// or the specified number of bytes have been read. Use the `response`
    /// object to set the mime type, http status code and other optional header
    /// values. To redirect the request to a new URL set `redirect_url` to the new
    /// URL. `redirect_url` can be either a relative or fully qualified URL. It is
    /// also possible to set `response` to a redirect http status code and pass the
    /// new URL via a Location header. Likewise with `redirect_url` it is valid to
    /// set a relative or fully qualified URL as the Location header value. If an
    /// error occured while setting up the request you can call [Response::set_error] on
    /// `response` to indicate the error condition.
    fn get_response_headers(&self, response: &mut Response, response_length: &mut i64, redirect_url: &mut String) {}
    /// Skip response data when requested by a Range header. Skip over and discard
    /// `bytes_to_skip` bytes of response data. If data is available immediately
    /// set `bytes_skipped` to the number of bytes skipped and return true. To
    /// read the data at a later time set `bytes_skipped` to 0, return true and
    /// execute `callback` when the data is available. To indicate failure set
    /// `bytes_skipped` to < 0 (e.g. -2 for [ErrorCode::Failed]) and return false. This
    /// function will be called in sequence but not from a dedicated thread.
    fn skip(&self, bytes_to_skip: i64, bytes_skipped: &mut i64, callback: Callback) -> bool { false }
    /// Read response data. If data is available immediately copy up to
    /// the slice len into `data_out`, set `bytes_read` to the number of
    /// bytes copied, and return true. To read the data at a later time keep a
    /// reference to `data_out`, set `bytes_read` to 0, return true and execute
    /// `callback` when the data is available (`data_out` will remain valid until
    /// the callback is executed). To indicate response completion set `bytes_read`
    /// to 0 and return false. To indicate failure set `bytes_read` to < 0
    /// (e.g. -2 for [ErrorCode::Failed]) and return false. This function will be called
    /// in sequence but not from a dedicated thread.
    fn read(&self, data_out: &mut [u8], bytes_read: &mut i32, callback: Callback) -> bool { false }
    /// Request processing has been canceled.
    fn cancel(&self) {}
}