use cef_sys::cef_drag_operations_mask_t;
use crate::{
    browser::{Browser, BrowserSettings, State},
    client::Client,
    devtools_message_observer::DevToolsMessageObserver,
    registration::Registration,
    drag::{DragData, DragOperation},
    events::{KeyEvent, MouseButtonType, MouseEvent, TouchEvent},
    extension::Extension,
    file_dialog::{FileDialogMode, RunFileDialogCallbackWrapper},
    image::Image,
    ime::CompositionUnderline,
    navigation::NavigationEntry,
    printing::PDFPrintSettings,
    refcounted::{RefCountedPtr, Wrapper},
    request_context::RequestContext,
    send_protector::SendProtectorMut,
    string::{CefString, CefStringList},
    values::{DictionaryValue, Point, Range, Size, StoredValue},
    window::{RawWindow, WindowInfo},
};
use cef_sys::{
    cef_browser_host_create_browser_sync, cef_browser_host_t,
    cef_download_image_callback_t, cef_image_t, cef_navigation_entry_t,
    cef_navigation_entry_visitor_t, cef_paint_element_type_t, cef_pdf_print_callback_t,
    cef_string_t,
};
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    iter::FromIterator,
    ptr::{null, null_mut},
};

/// Paint element types.
#[repr(C)]
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum PaintElementType {
    View = cef_paint_element_type_t::PET_VIEW as isize,
    Popup = cef_paint_element_type_t::PET_POPUP as isize,
}

impl PaintElementType {
    pub unsafe fn from_unchecked(c: crate::CEnumType) -> Self {
        std::mem::transmute(c)
    }
}

ref_counted_ptr! {
    /// Structure used to represent the browser process aspects of a browser window.
    /// The functions of this structure can only be called in the browser process.
    /// They may be called on any thread in that process unless otherwise indicated
    /// in the comments.
    pub struct BrowserHost(*mut cef_browser_host_t);
}

impl BrowserHost {
    // TODO: HOW CAN WE MAKE THIS SAFE?
    // We've run into memory issues when using this function and `BrowserHost::invalidate`. It's
    // difficult to guarantee that everything was initialized properly before `invalidate` gets
    // called, and while we don't know how to do that it seems safer to just disable this function.
    // /// Create a new browser window using the window parameters specified by
    // /// `window_info`. All values will be copied internally and the actual window will
    // /// be created on the UI thread. If `request_context` is None the global request
    // /// context will be used. This function can be called on any browser process
    // /// thread and will not block. The optional `extra_info` parameter provides an
    // /// opportunity to specify extra information specific to the created browser that
    // /// will be passed to [RenderProcessHandlerCallbacks::on_browser_created] in the
    // /// render process.
    // pub fn create_browser(
    //     window_info: &WindowInfo,
    //     client: Client,
    //     url: &str,
    //     settings: &BrowserSettings,
    //     extra_info: Option<&HashMap<String, StoredValue>>,
    //     request_context: Option<&RequestContext>,
    // ) -> bool {
    //     let extra_info = extra_info.map(DictionaryValue::from);

    //     unsafe {
    //         cef_browser_host_create_browser(
    //             &window_info.into_raw(),
    //             client.into_raw(),
    //             CefString::new(url).as_ptr(),
    //             &settings.into_raw(),
    //             extra_info.map(|ei| ei.as_ptr()).unwrap_or_else(null_mut),
    //             request_context
    //                 .map(|rc| rc.as_ptr())
    //                 .unwrap_or_else(null_mut),
    //         ) != 0
    //     }
    // }
    /// Create a new browser window using the window parameters specified by
    /// `windowInfo`. If `request_context` is None the global request context will be
    /// used. This function can only be called on the browser process UI thread. The
    /// optional `extra_info` parameter provides an opportunity to specify extra
    /// information specific to the created browser that will be passed to
    /// [RenderProcessHandlerCallbacks::on_browser_created] in the render process.
    pub fn create_browser_sync(
        window_info: &WindowInfo,
        client: Client,
        url: &str,
        settings: &BrowserSettings,
        extra_info: Option<&HashMap<String, StoredValue>>,
        request_context: Option<RequestContext>,
    ) -> Browser {
        let extra_info = extra_info.map(DictionaryValue::from);

        unsafe {
            Browser::from_ptr_unchecked(cef_browser_host_create_browser_sync(
                &window_info.into_raw(),
                client.into_raw(),
                CefString::new(url).as_ptr(),
                &settings.into_raw(),
                extra_info.map(|ei| ei.as_ptr()).unwrap_or_else(null_mut),
                request_context
                    .map(|rc| rc.into_raw())
                    .unwrap_or_else(null_mut),
            ))
        }
    }
    /// Returns the hosted browser object.
    pub fn get_browser(&self) -> Browser {
        unsafe { Browser::from_ptr_unchecked(self.0.get_browser.unwrap()(self.0.as_ptr())) }
    }
    /// Request that the browser close. The JavaScript 'onbeforeunload' event will
    /// be fired. If `force_close` is false the event handler, if any, will be
    /// allowed to prompt the user and the user can optionally cancel the close. If
    /// `force_close` is true the prompt will not be displayed and the close
    /// will proceed. Results in a call to [LifeSpanHandler::do_close] if
    /// the event handler allows the close or if `force_close` is true. See
    /// [LifeSpanHandler::do_close] documentation for additional usage
    /// information.
    pub fn close_browser(&self, force_close: bool) {
        if let Some(close_browser) = self.0.close_browser {
            unsafe {
                close_browser(self.0.as_ptr(), force_close as i32);
            }
        }
    }
    /// Helper for closing a browser. Call this function from the top-level window
    /// close handler. Internally this calls CloseBrowser(false) if the close
    /// has not yet been initiated. This function returns false while the close
    /// is pending and true after the close has completed. See [close_browser]
    /// and [LifeSpanHandler::do_close] documentation for additional usage
    /// information. This function must be called on the browser process UI thread.
    pub fn try_close_browser(&self) -> bool {
        self.0
            .try_close_browser
            .map(|try_close_browser| unsafe { try_close_browser(self.0.as_ptr()) != 0 })
            .unwrap_or(false)
    }
    /// Set whether the browser is focused.
    pub fn set_focus(&self, focus: bool) {
        if let Some(set_focus) = self.0.set_focus {
            unsafe {
                set_focus(self.0.as_ptr(), focus as i32);
            }
        }
    }
    /// Retrieve the window handle for this browser. If this browser is wrapped in
    /// a [BrowserView] this function should be called on the browser process
    /// UI thread and it will return the handle for the top-level native window.
    pub fn get_window_handle(&self) -> Option<RawWindow> {
        self.0
            .get_window_handle
            .and_then(|get_window_handle| unsafe { RawWindow::from_cef_handle(get_window_handle(self.0.as_ptr())) })
    }
    /// Retrieve the window handle of the browser that opened this browser. Will
    /// return None for non-popup windows or if this browser is wrapped in a
    /// [BrowserView]. This function can be used in combination with custom
    /// handling of modal windows.
    pub fn get_opener_window_handle(&self) -> Option<RawWindow> {
        self.0
            .get_opener_window_handle
            .and_then(|get_opener_window_handle| unsafe { RawWindow::from_cef_handle(get_opener_window_handle(self.0.as_ptr())) })
    }
    /// Returns true if this browser is wrapped in a [BrowserView].
    pub fn has_view(&self) -> bool {
        self.0
            .has_view
            .map(|has_view| unsafe { has_view(self.0.as_ptr()) != 0 })
            .unwrap_or(false)
    }
    /// Returns the client for this browser, None if the type is not correct.
    pub fn get_client(&self) -> Option<Client> {
        let get_client = self.0.get_client.unwrap();
        unsafe{ Client::from_ptr(get_client(self.0.as_ptr())) }
    }
    /// Returns the request context for this browser.
    pub fn get_request_context(&self) -> RequestContext {
        self.0
            .get_request_context
            .and_then(|get_request_context| unsafe {
                RequestContext::from_ptr(get_request_context(self.0.as_ptr()))
            })
            .unwrap()
    }
    /// Get the current zoom level. The default zoom level is 0.0. This function
    /// can only be called on the UI thread.
    pub fn get_zoom_level(&self) -> f64 {
        self.0
            .get_zoom_level
            .map(|get_zoom_level| unsafe { get_zoom_level(self.0.as_ptr()) })
            .unwrap_or(0.0)
    }
    /// Change the zoom level to the specified value. Specify 0.0 to reset the zoom
    /// level. If called on the UI thread the change will be applied immediately.
    /// Otherwise, the change will be applied asynchronously on the UI thread.
    pub fn set_zoom_level(&self, zoom_level: f64) {
        if let Some(set_zoom_level) = self.0.set_zoom_level {
            unsafe {
                set_zoom_level(self.0.as_ptr(), zoom_level);
            }
        }
    }
    /// Call to run a file chooser dialog. Only a single file chooser dialog may be
    /// pending at any given time. `mode` represents the type of dialog to display.
    /// `title` to the title to be used for the dialog and may be None to show the
    /// default title ("Open" or "Save" depending on the mode). `default_file_path`
    /// is the path with optional directory and/or file name component that will be
    /// initially selected in the dialog. `accept_filters` are used to restrict the
    /// selectable file types and may any combination of (a) valid lower-cased MIME
    /// types (e.g. "text/*" or "image/\*"), (b) individual file extensions (e.g.
    /// ".txt" or ".png"), or (c) combined description and file extension delimited
    /// using "|" and ";" (e.g. "Image Types|.png;.gif;.jpg").
    /// `selected_accept_filter` is the 0-based index of the filter that will be
    /// selected by default. `callback` will be executed after the dialog is
    /// dismissed or immediately if another dialog is already pending. The dialog
    /// will be initiated asynchronously on the UI thread.
    ///
    /// On the `callback`, the first parameter is the 0-based index of the value
    /// selected from `accept_filters`. The second parameter will be a single value
    /// or a list of values depending on the dialog mode. If the selection was
    /// cancelled it will be None.
    pub fn run_file_dialog(
        &self,
        mode: FileDialogMode,
        title: Option<&str>,
        default_file_path: Option<&str>,
        accept_filters: &[&str],
        selected_accept_filter: i32,
        callback: impl Send + FnOnce(usize, Option<Vec<String>>) + 'static,
    ) {
        if let Some(run_file_dialog) = self.0.run_file_dialog {
            let title = title.map(CefString::new);
            let default_file_path = default_file_path.map(CefString::new);
            unsafe {
                run_file_dialog(
                    self.0.as_ptr(),
                    mode.into(),
                    title.map(|s| s.as_ptr()).unwrap_or_else(null),
                    default_file_path.map(|s| s.as_ptr()).unwrap_or_else(null),
                    CefStringList::from_iter(accept_filters.iter().cloned()).into_raw(),
                    selected_accept_filter,
                    RunFileDialogCallbackWrapper::new(callback)
                        .wrap()
                        .into_raw(),
                );
            }
        }
    }
    /// Download the file at `url` using [DownloadHandler].
    pub fn start_download(&self, url: &str) {
        if let Some(start_download) = self.0.start_download {
            unsafe {
                start_download(self.0.as_ptr(), CefString::new(url).as_ptr());
            }
        }
    }
    /// Download `image_url` and execute `callback` on completion with the images
    /// received from the renderer. If `is_favicon` is true then cookies are
    /// not sent and not accepted during download. Images with density independent
    /// pixel (DIP) sizes larger than `max_image_size` are filtered out from the
    /// image results. Versions of the image at different scale factors may be
    /// downloaded up to the maximum scale factor supported by the system. If there
    /// are no image results <= `max_image_size` then the smallest image is resized
    /// to `max_image_size` and is the only result. A `max_image_size` of 0 means
    /// unlimited. If `bypass_cache` is true then `image_url` is requested from
    /// the server even if it is present in the browser cache.
    ///
    /// On the callback, the first parameter is the URL that was downloaded, the
    /// second parameter is the resulting HTTP status code and the third is the
    /// resulting image, possibly None if the download failed. It will be called
    /// on the browser process UI thread.
    pub fn download_image(
        &self,
        image_url: &str,
        is_favicon: bool,
        max_image_size: u32,
        bypass_cache: bool,
        callback: impl Send + FnOnce(&str, u16, Option<Image>) + 'static,
    ) {
        if let Some(download_image) = self.0.download_image {
            unsafe {
                download_image(
                    self.0.as_ptr(),
                    CefString::new(image_url).as_ptr(),
                    is_favicon as i32,
                    max_image_size,
                    bypass_cache as i32,
                    DownloadImageCallbackWrapper::new(callback)
                        .wrap()
                        .into_raw(),
                );
            }
        }
    }
    /// Print the current browser contents.
    pub fn print(&self) {
        if let Some(print) = self.0.print {
            unsafe {
                print(self.0.as_ptr());
            }
        }
    }
    /// Print the current browser contents to the PDF file specified by `path` and
    /// execute `callback` on completion. The caller is responsible for deleting
    /// `path` when done. For PDF printing to work on Linux you must implement the
    /// [PrintHandler::GetPdfPaperSize] function.
    ///
    /// On the callback, the first parameter is the output path. The second parameter
    /// will be true if the printing completed successfully or false otherwise. It
    /// will be called on the browser process UI thread.
    pub fn print_to_pdf(
        &self,
        path: &str,
        settings: &PDFPrintSettings,
        callback: impl Send + FnOnce(&str, bool) + 'static,
    ) {
        if let Some(print_to_pdf) = self.0.print_to_pdf {
            unsafe {
                print_to_pdf(
                    self.0.as_ptr(),
                    CefString::new(path).as_ptr(),
                    &settings.into(),
                    PDFPrintCallbackWrapper::new(callback).wrap().into_raw(),
                );
            }
        }
    }
    /// Search for `searchText`. `identifier` must be a unique ID and these IDs
    /// must strictly increase so that newer requests always have greater IDs than
    /// older requests. If `identifier` is zero or less than the previous ID value
    /// then it will be automatically assigned a new valid ID. `forward` indicates
    /// whether to search forward or backward within the page. `match_case`
    /// indicates whether the search should be case-sensitive. `find_next` indicates
    /// whether this is the first request or a follow-up. The [FindHandler]
    /// instance, if any, returned via [ClientCallbacks::get_find_handler] will be called
    /// to report find results.
    pub fn find(
        &self,
        identifier: i32,
        search_text: &str,
        forward: bool,
        match_case: bool,
        find_next: bool,
    ) {
        if let Some(find) = self.0.find {
            unsafe {
                find(
                    self.0.as_ptr(),
                    identifier,
                    CefString::new(search_text).as_ptr(),
                    forward as i32,
                    match_case as i32,
                    find_next as i32,
                );
            }
        }
    }
    /// Cancel all searches that are currently going on.
    pub fn stop_finding(&self, clear_selection: bool) {
        if let Some(stop_finding) = self.0.stop_finding {
            unsafe {
                stop_finding(self.0.as_ptr(), clear_selection as i32);
            }
        }
    }
    /// Open developer tools (DevTools) in its own browser. The DevTools browser
    /// will remain associated with this browser. If the DevTools browser is
    /// already open then it will be focused, in which case the `window_info`,
    /// `client` and `settings` parameters will be ignored. If `inspect_element_at`
    /// is non-None then the element at the specified (x,y) location will be
    /// inspected. The `window_info` parameter will be ignored if this browser is
    /// wrapped in a [BrowserView].
    pub fn show_dev_tools(
        &self,
        window_info: &WindowInfo,
        client: Option<Client>,
        settings: Option<BrowserSettings>,
        inspect_element_at: Point,
    ) {
        if let Some(show_dev_tools) = self.0.show_dev_tools {
            let client = client
                .map(Client::into_raw)
                .unwrap_or_else(null_mut);
            let settings = settings.map(|s| s.into_raw());
            unsafe {
                show_dev_tools(
                    self.0.as_ptr(),
                    &window_info.into_raw(),
                    client,
                    settings.map(|s| &s as *const _).unwrap_or_else(null),
                    &inspect_element_at.into(),
                );
            }
        }
    }
    /// Explicitly close the associated DevTools browser, if any.
    pub fn close_dev_tools(&self) {
        if let Some(close_dev_tools) = self.0.close_dev_tools {
            unsafe {
                close_dev_tools(self.0.as_ptr());
            }
        }
    }
    /// Returns true if this browser currently has an associated DevTools
    /// browser. Must be called on the browser process UI thread.
    pub fn has_dev_tools(&self) -> bool {
        self.0
            .has_dev_tools
            .map(|has_dev_tools| unsafe { has_dev_tools(self.0.as_ptr()) != 0 })
            .unwrap_or(false)
    }
    /// Send a function call message over the DevTools protocol. `message` must be
    /// a UTF8-encoded JSON dictionary that contains `id` (int), `function`
    /// (string) and `params` (dictionary, optional) values. See the DevTools
    /// protocol documentation at https://chromedevtools.github.io/devtools-protocol/
    /// for details of supported functions and the expected `params`
    /// dictionary contents. `message` will be copied if necessary. This function
    /// will return `true` if called on the UI thread and the message was
    /// successfully submitted for validation, otherwise `false`. Validation will
    /// be applied asynchronously and any messages that fail due to formatting
    /// errors or missing parameters may be discarded without notification. Prefer
    /// `execute_dev_tools_method` if a more structured approach to message formatting
    /// is desired.
    ///
    /// Every valid function call will result in an asynchronous function result or
    /// error message that references the sent message `id`. Event messages are
    /// received while notifications are enabled (for example, between function
    /// calls for `Page.enable` and `Page.disable`). All received messages will be
    /// delivered to the observer(s) registered with `add_dev_tools_message_observer`.
    /// See [`DevToolsMessageObserverCallbacks::on_dev_tools_message`] documentation for
    /// details of received message contents.
    ///
    /// Usage of the `send_dev_tools_message`, `execute_dev_tools_method` and
    /// `add_dev_tools_message_observer` functions does not require an active DevTools
    /// front-end or remote-debugging session. Other active DevTools sessions will
    /// continue to function independently. However, any modification of global
    /// browser state by one session may not be reflected in the UI of other
    /// sessions.
    ///
    /// Communication with the DevTools front-end (when displayed) can be logged
    /// for development purposes by passing the `--devtools-protocol-log-
    /// file=<path>` command-line flag.
    pub fn send_dev_tools_message(
        &self,
        message: &[u8],
    ) -> bool {
        unsafe {
            self.0.send_dev_tools_message.unwrap()(
                self.as_ptr(),
                message.as_ptr() as *const _,
                message.len(),
            ) != 0
        }
    }
    /// Execute a function call over the DevTools protocol. This is a more
    /// structured version of `send_dev_tools_message`. `message_id` is an incremental
    /// number that uniquely identifies the message (pass 0 to have the next number
    /// assigned automatically based on previous values). `function` is the
    /// function name. `params` are the function parameters, which may be `None`. See
    /// the DevTools protocol documentation (linked above) for details of supported
    /// functions and the expected `params` dictionary contents. This function will
    /// return the assigned message ID if called on the UI thread and the message
    /// was successfully submitted for validation, otherwise 0. See the
    /// `send_dev_tools_message` documentation for additional usage information.
    pub fn execute_dev_tools_method(
        &self,
        message_id: i32,
        method: &str,
        params: Option<DictionaryValue>,
    ) -> bool {
        unsafe {
            self.0.execute_dev_tools_method.unwrap()(
                self.as_ptr(),
                message_id,
                CefString::from(method).as_ptr(),
                params.map(|p| p.into_raw()).unwrap_or_else(null_mut),
            ) != 0
        }
    }
    /// Add an observer for DevTools protocol messages (function results and
    /// events). The observer will remain registered until the returned
    /// Registration object is destroyed. See the `send_dev_tools_message` documentation
    /// for additional usage information.
    pub fn add_dev_tools_message_observer(
        &self,
        observer: DevToolsMessageObserver,
    ) -> Registration {
        unsafe {
            Registration::from_ptr_unchecked(
                self.0.add_dev_tools_message_observer.unwrap()(
                    self.as_ptr(),
                    observer.into_raw()
                )
            )
        }
    }
    /// Retrieve a snapshot of current navigation entries as values sent to the
    /// specified visitor. If `current_only` is true only the current
    /// navigation entry will be sent, otherwise all navigation entries will be
    /// sent.
    ///
    /// The visitor will be called on the browser process UI thread.
    pub fn get_navigation_entries(
        &self,
        visitor: NavigationEntryVisitor,
        current_only: bool,
    ) {
        if let Some(get_navigation_entries) = self.0.get_navigation_entries {
            unsafe {
                get_navigation_entries(
                    self.0.as_ptr(),
                    visitor.into_raw(),
                    current_only as i32,
                );
            }
        }
    }
    /// Set whether mouse cursor change is disabled.
    pub fn set_mouse_cursor_change_disabled(&self, disabled: bool) {
        if let Some(set_mouse_cursor_change_disabled) = self.0.set_mouse_cursor_change_disabled {
            unsafe {
                set_mouse_cursor_change_disabled(self.0.as_ptr(), disabled as i32);
            }
        }
    }
    /// Returns true if mouse cursor change is disabled.
    pub fn is_mouse_cursor_change_disabled(&self) -> bool {
        self.0
            .is_mouse_cursor_change_disabled
            .map(|is_mouse_cursor_change_disabled| unsafe {
                is_mouse_cursor_change_disabled(self.0.as_ptr()) != 0
            })
            .unwrap_or(false)
    }
    /// If a misspelled word is currently selected in an editable node calling this
    /// function will replace it with the specified `word`.
    pub fn replace_misspelling(&self, word: &str) {
        if let Some(replace_misspelling) = self.0.replace_misspelling {
            unsafe {
                replace_misspelling(self.0.as_ptr(), CefString::new(word).as_ptr());
            }
        }
    }
    /// Add the specified `word` to the spelling dictionary.
    pub fn add_word_to_dictionary(&self, word: &str) {
        if let Some(add_word_to_dictionary) = self.0.add_word_to_dictionary {
            unsafe {
                add_word_to_dictionary(self.0.as_ptr(), CefString::new(word).as_ptr());
            }
        }
    }
    /// Returns true if window rendering is disabled.
    pub fn is_window_rendering_disabled(&self) -> bool {
        self.0
            .is_window_rendering_disabled
            .map(|is_window_rendering_disabled| unsafe {
                is_window_rendering_disabled(self.0.as_ptr()) != 0
            })
            .unwrap_or(false)
    }
    /// Notify the browser that the widget has been resized. The browser will first
    /// call [RenderHandler::get_view_rect] to get the new size and then call
    /// [RenderHandler::on_paint] asynchronously with the updated regions. This
    /// function is only used when window rendering is disabled.
    pub fn was_resized(&self) {
        if let Some(was_resized) = self.0.was_resized {
            unsafe {
                was_resized(self.0.as_ptr());
            }
        }
    }
    /// Notify the browser that it has been hidden or shown. Layouting and
    /// [RenderHandler::on_paint] notification will stop when the browser is
    /// hidden. This function is only used when window rendering is disabled.
    pub fn was_hidden(&self, hidden: bool) {
        if let Some(was_hidden) = self.0.was_hidden {
            unsafe {
                was_hidden(self.0.as_ptr(), hidden as i32);
            }
        }
    }
    /// Send a notification to the browser that the screen info has changed. The
    /// browser will then call [RenderHandler::get_screen_info] to update the
    /// screen information with the new values. This simulates moving the webview
    /// window from one display to another, or changing the properties of the
    /// current display. This function is only used when window rendering is
    /// disabled.
    pub fn notify_screen_info_changed(&self) {
        if let Some(notify_screen_info_changed) = self.0.notify_screen_info_changed {
            unsafe {
                notify_screen_info_changed(self.0.as_ptr());
            }
        }
    }
    /// Invalidate the view. The browser will call [RenderHandler::on_paint]
    /// asynchronously. This function is only used when window rendering is
    /// disabled.
    pub fn invalidate(&self, element_type: PaintElementType) {
        if let Some(invalidate) = self.0.invalidate {
            unsafe {
                invalidate(self.0.as_ptr(), element_type as crate::CEnumType);
            }
        }
    }
    /// Issue a BeginFrame request to Chromium.  Only valid when
    /// [WindowInfo::external_begin_frame_enabled] is set to true.
    pub fn send_external_begin_frame(&self) {
        if let Some(send_external_begin_frame) = self.0.send_external_begin_frame {
            unsafe {
                send_external_begin_frame(self.0.as_ptr());
            }
        }
    }
    /// Send a key event to the browser.
    pub fn send_key_event(&self, event: KeyEvent) {
        if let Some(send_key_event) = self.0.send_key_event {
            unsafe {
                send_key_event(self.0.as_ptr(), &event.as_cef());
            }
        }
    }
    /// Send a mouse click event to the browser. The `x` and `y` coordinates are
    /// relative to the upper-left corner of the view.
    pub fn send_mouse_click_event(
        &self,
        event: &MouseEvent,
        button_type: MouseButtonType,
        mouse_up: bool,
        click_count: i32,
    ) {
        if let Some(send_mouse_click_event) = self.0.send_mouse_click_event {
            unsafe {
                send_mouse_click_event(
                    self.0.as_ptr(),
                    event.as_cef(),
                    button_type as crate::CEnumType,
                    mouse_up as i32,
                    click_count,
                );
            }
        }
    }
    /// Send a mouse move event to the browser. The `x` and `y` coordinates are
    /// relative to the upper-left corner of the view.
    pub fn send_mouse_move_event(&self, event: &MouseEvent, mouse_leave: bool) {
        if let Some(send_mouse_move_event) = self.0.send_mouse_move_event {
            unsafe {
                send_mouse_move_event(self.0.as_ptr(), event.as_cef(), mouse_leave as i32);
            }
        }
    }
    /// Send a mouse wheel event to the browser. The `x` and `y` coordinates are
    /// relative to the upper-left corner of the view. The `deltaX` and `deltaY`
    /// values represent the movement delta in the X and Y directions respectively.
    /// In order to scroll inside select popups with window rendering disabled
    /// [RenderHandler::get_screen_point] should be implemented properly.
    pub fn send_mouse_wheel_event(&self, event: &MouseEvent, delta_x: i32, delta_y: i32) {
        if let Some(send_mouse_wheel_event) = self.0.send_mouse_wheel_event {
            unsafe {
                send_mouse_wheel_event(self.0.as_ptr(), event.as_cef(), delta_x, delta_y);
            }
        }
    }
    /// Send a touch event to the browser for a windowless browser.
    pub fn send_touch_event(&self, event: &TouchEvent) {
        if let Some(send_touch_event) = self.0.send_touch_event {
            unsafe {
                send_touch_event(self.0.as_ptr(), event.as_cef());
            }
        }
    }
    /// Send a focus event to the browser.
    pub fn send_focus_event(&self, set_focus: bool) {
        if let Some(send_focus_event) = self.0.send_focus_event {
            unsafe {
                send_focus_event(self.0.as_ptr(), set_focus as i32);
            }
        }
    }
    /// Send a capture lost event to the browser.
    pub fn send_capture_lost_event(&self) {
        if let Some(send_capture_lost_event) = self.0.send_capture_lost_event {
            unsafe {
                send_capture_lost_event(self.0.as_ptr());
            }
        }
    }
    /// Notify the browser that the window hosting it is about to be moved or
    /// resized. This function is only used on Windows and Linux.
    pub fn notify_move_or_resize_started(&self) {
        if let Some(notify_move_or_resize_started) = self.0.notify_move_or_resize_started {
            unsafe {
                notify_move_or_resize_started(self.0.as_ptr());
            }
        }
    }
    /// Returns the maximum rate in frames per second (fps) that
    /// [RenderHandler::on_paint] will be called for a windowless browser. The
    /// actual fps may be lower if the browser cannot generate frames at the
    /// requested rate. The minimum value is 1 and the maximum value is 60 (default
    /// 30). This function can only be called on the UI thread.
    pub fn get_windowless_frame_rate(&self) -> i32 {
        self.0
            .get_windowless_frame_rate
            .map(|get_windowless_frame_rate| unsafe { get_windowless_frame_rate(self.0.as_ptr()) })
            .unwrap_or(30)
    }
    /// Set the maximum rate in frames per second (fps) that [RenderHandler::on_paint]
    /// will be called for a windowless browser. The actual fps may be
    /// lower if the browser cannot generate frames at the requested rate. The
    /// minimum value is 1 and the maximum value is 60 (default 30). Can also be
    /// set at browser creation via [BrowserSettings::windowless_frame_rate].
    pub fn set_windowless_frame_rate(&self, frame_rate: i32) {
        if let Some(set_windowless_frame_rate) = self.0.set_windowless_frame_rate {
            unsafe {
                set_windowless_frame_rate(self.0.as_ptr(), frame_rate);
            }
        }
    }
    /// Begins a new composition or updates the existing composition. Blink has a
    /// special node (a composition node) that allows the input function to change
    /// text without affecting other DOM nodes. `text` is the optional text that
    /// will be inserted into the composition node. `underlines` is an optional set
    /// of ranges that will be underlined in the resulting text.
    /// `replacement_range` is an optional range of the existing text that will be
    /// replaced. `selection_range` is an optional range of the resulting text that
    /// will be selected after insertion or replacement. The `replacement_range`
    /// value is only used on OS X.
    ///
    /// This function may be called multiple times as the composition changes. When
    /// the client is done making changes the composition should either be canceled
    /// or completed. To cancel the composition call [BrowserHost::ime_cancel_composition]. To
    /// complete the composition call either [BrowserHost::ime_commit_text] or
    /// [BrowserHost::ime_finish_composing_text]. Completion is usually signaled when:
    ///   A. The client receives a WM_IME_COMPOSITION message with a GCS_RESULTSTR
    ///      flag (on Windows), or;
    ///   B. The client receives a "commit" signal of GtkIMContext (on Linux), or;
    ///   C. insertText of NSTextInput is called (on Mac).
    ///
    /// This function is only used when window rendering is disabled.
    pub fn ime_set_composition(
        &self,
        text: &str,
        underlines_count: usize,
        underlines: &CompositionUnderline,
        replacement_range: &Range,
        selection_range: &Range,
    ) {
        if let Some(ime_set_composition) = self.0.ime_set_composition {
            unsafe {
                ime_set_composition(
                    self.0.as_ptr(),
                    CefString::new(text).as_ptr(),
                    underlines_count,
                    &underlines.into(),
                    replacement_range.as_ptr(),
                    selection_range.as_ptr(),
                );
            }
        }
    }

    /// Completes the existing composition by optionally inserting the specified
    /// `text` into the composition node. `replacement_range` is an optional range
    /// of the existing text that will be replaced. `relative_cursor_pos` is where
    /// the cursor will be positioned relative to the current cursor position. See
    /// comments on [BrowserHost::ime_set_composition] for usage. The `replacement_range` and
    /// `relative_cursor_pos` values are only used on OS X. This function is only
    /// used when window rendering is disabled.
    pub fn ime_commit_text(
        &self,
        text: Option<&str>,
        replacement_range: Option<&Range>,
        relative_cursor_pos: i32,
    ) {
        if let Some(ime_commit_text) = self.0.ime_commit_text {
            let text = text.map(|text| CefString::new(text));
            unsafe {
                ime_commit_text(
                    self.0.as_ptr(),
                    text.map(|s| s.as_ptr()).unwrap_or_else(null),
                    replacement_range.map(Range::as_ptr).unwrap_or_else(null),
                    relative_cursor_pos,
                );
            }
        }
    }
    /// Completes the existing composition by applying the current composition node
    /// contents. If `keep_selection` is false the current selection, if any,
    /// will be discarded. See comments on [BrowserHost::ime_set_composition] for usage. This
    /// function is only used when window rendering is disabled.
    pub fn ime_finish_composing_text(&self, keep_selection: bool) {
        if let Some(ime_finish_composing_text) = self.0.ime_finish_composing_text {
            unsafe {
                ime_finish_composing_text(self.0.as_ptr(), keep_selection as i32);
            }
        }
    }
    /// Cancels the existing composition and discards the composition node contents
    /// without applying them. See comments on ImeSetComposition for usage. This
    /// function is only used when window rendering is disabled.
    pub fn ime_cancel_composition(&self) {
        if let Some(ime_cancel_composition) = self.0.ime_cancel_composition {
            unsafe {
                ime_cancel_composition(self.0.as_ptr());
            }
        }
    }
    /// Call this function when the user drags the mouse into the web view (before
    /// calling [BrowserHost::drag_target_drag_over]/[BrowserHost::drag_target_leave]/[BrowserHost::drag_target_drop]). `drag_data`
    /// should not contain file contents as this type of data is not allowed to be
    /// dragged into the web view. File contents can be removed using
    /// [DragData::reset_file_contents] (for example, if `drag_data` comes from
    /// [RenderHandler::start_dragging]). This function is only used when
    /// window rendering is disabled.
    pub fn drag_target_drag_enter(
        &self,
        drag_data: DragData,
        event: &MouseEvent,
        allowed_ops: DragOperation,
    ) {
        if let Some(drag_target_drag_enter) = self.0.drag_target_drag_enter {
            unsafe {
                drag_target_drag_enter(
                    self.0.as_ptr(),
                    drag_data.into_raw(),
                    event.as_cef(),
                    cef_drag_operations_mask_t(allowed_ops.bits()),
                );
            }
        }
    }
    /// Call this function each time the mouse is moved across the web view during
    /// a drag operation (after calling [BrowserHost::drag_target_drag_enter] and before calling
    /// [BrowserHost::drag_target_drag_leave]/[BrowserHost::drag_target_drop]). This function is only used when window
    /// rendering is disabled.
    pub fn drag_target_drag_over(&self, event: &MouseEvent, allowed_ops: DragOperation) {
        if let Some(drag_target_drag_over) = self.0.drag_target_drag_over {
            unsafe {
                drag_target_drag_over(
                    self.0.as_ptr(),
                    event.as_cef(),
                    cef_drag_operations_mask_t(allowed_ops.bits()),
                );
            }
        }
    }
    /// Call this function when the user drags the mouse out of the web view (after
    /// calling [BrowserHost::drag_target_drag_enter]). This function is only used when window
    /// rendering is disabled.
    pub fn drag_target_drag_leave(&self) {
        if let Some(drag_target_drag_leave) = self.0.drag_target_drag_leave {
            unsafe {
                drag_target_drag_leave(self.0.as_ptr());
            }
        }
    }
    /// Call this function when the user completes the drag operation by dropping
    /// the object onto the web view (after calling [BrowserHost::drag_target_drag_enter]). The
    /// object being dropped is `drag_data`, given as an argument to the previous
    /// [BrowserHost::drag_target_drag_enter] call. This function is only used when window rendering
    /// is disabled.
    pub fn drag_target_drop(&self, event: &MouseEvent) {
        if let Some(drag_target_drop) = self.0.drag_target_drop {
            unsafe {
                drag_target_drop(self.0.as_ptr(), event.as_cef());
            }
        }
    }
    /// Call this function when the drag operation started by a
    /// [RenderHandler::start_dragging] call has ended either in a drop or by
    /// being cancelled. `x` and `y` are mouse coordinates relative to the upper-
    /// left corner of the view. If the web view is both the drag source and the
    /// drag target then all drag_target_* functions should be called before
    /// drag_source_* methods. This function is only used when window rendering is
    /// disabled.
    pub fn drag_source_ended_at(&self, x: i32, y: i32, op: DragOperation) {
        if let Some(drag_source_ended_at) = self.0.drag_source_ended_at {
            unsafe {
                drag_source_ended_at(self.0.as_ptr(), x, y, cef_drag_operations_mask_t(op.bits()));
            }
        }
    }
    /// Call this function when the drag operation started by a
    /// [RenderHandler::start_dragging] call has completed. This function may
    /// be called immediately without first calling [BrowserHost::drag_source_ended_at] to cancel a
    /// drag operation. If the web view is both the drag source and the drag target
    /// then all drag_target_* functions should be called before drag_source_* methods.
    /// This function is only used when window rendering is disabled.
    pub fn drag_source_system_drag_ended(&self) {
        if let Some(drag_source_system_drag_ended) = self.0.drag_source_system_drag_ended {
            unsafe {
                drag_source_system_drag_ended(self.0.as_ptr());
            }
        }
    }
    /// Returns the current visible navigation entry for this browser. This
    /// function can only be called on the UI thread.
    pub fn get_visible_navigation_entry(&self) -> NavigationEntry {
        let get_visible_navigation_entry = self.0.get_visible_navigation_entry.unwrap();
        unsafe {
            NavigationEntry::from_ptr_unchecked(get_visible_navigation_entry(self.0.as_ptr()))
        }
    }
    /// Set accessibility state for all frames. If `accessibility_state` is [State::Default]
    /// then accessibility will be disabled by default and the state may be further
    /// controlled with the "force-renderer-accessibility" and "disable-renderer-
    /// accessibility" command-line switches. If `accessibility_state` is
    /// [State::Enabled] then accessibility will be enabled. If `accessibility_state
    /// is [State::Disabled] then accessibility will be completely disabled.
    ///
    /// For windowed browsers accessibility will be enabled in Complete mode (which
    /// corresponds to `kAccessibilityModeComplete` in Chromium). In this mode all
    /// platform accessibility objects will be created and managed by Chromium's
    /// internal implementation. The client needs only to detect the screen reader
    /// and call this function appropriately. For example, on macOS the client can
    /// handle the `@"AXEnhancedUserStructure"` accessibility attribute to detect
    /// VoiceOver state changes and on Windows the client can handle `WM_GETOBJECT`
    /// with `OBJID_CLIENT` to detect accessibility readers.
    ///
    /// For windowless browsers accessibility will be enabled in TreeOnly mode
    /// (which corresponds to `kAccessibilityModeWebContentsOnly` in Chromium). In
    /// this mode renderer accessibility is enabled, the full tree is computed, and
    /// events are passed to [AccessibiltyHandler], but platform accessibility
    /// objects are not created. The client may implement platform accessibility
    /// objects using [AccessibiltyHandler] callbacks if desired.
    pub fn set_accessibility_state(&self, accessibility_state: State) {
        if let Some(set_accessibility_state) = self.0.set_accessibility_state {
            unsafe {
                set_accessibility_state(self.0.as_ptr(), accessibility_state as crate::CEnumType);
            }
        }
    }
    /// Enable notifications of auto resize via
    /// [DisplayHandler::on_auto_resize]. Notifications are disabled by default.
    /// `min_size` and `max_size` define the range of allowed sizes.
    pub fn set_auto_resize_enabled(&self, enabled: bool, min_size: &Size, max_size: &Size) {
        if let Some(set_auto_resize_enabled) = self.0.set_auto_resize_enabled {
            unsafe {
                set_auto_resize_enabled(
                    self.0.as_ptr(),
                    enabled as i32,
                    min_size.as_ptr(),
                    max_size.as_ptr(),
                );
            }
        }
    }
    /// Returns the extension hosted in this browser or None if no extension is
    /// hosted. See [RequestContest::load_extension] for details.
    pub fn get_extension(&self) -> Option<Extension> {
        self.0.get_extension.and_then(|get_extension| unsafe {
            Extension::from_ptr(get_extension(self.0.as_ptr()))
        })
    }
    /// Returns true if this browser is hosting an extension background script.
    /// Background hosts do not have a window and are not displayable. See
    /// [RequestContext::load_extension] for details.
    pub fn is_background_host(&self) -> bool {
        self.0
            .is_background_host
            .map(|is_background_host| unsafe { is_background_host(self.0.as_ptr()) != 0 })
            .unwrap_or(false)
    }
    ///  Set whether the browser's audio is muted.
    pub fn set_audio_muted(&self, mute: bool) {
        if let Some(set_audio_muted) = self.0.set_audio_muted {
            unsafe {
                set_audio_muted(self.0.as_ptr(), mute as i32);
            }
        }
    }
    /// Returns true if the browser's audio is muted. This function can only
    /// be called on the UI thread.
    pub fn is_audio_muted(&self) -> bool {
        self.0
            .is_audio_muted
            .map(|is_audio_muted| unsafe { is_audio_muted(self.0.as_ptr()) != 0 })
            .unwrap_or(false)
    }
}

pub(crate) struct DownloadImageCallbackWrapper {
    callback: Mutex<Option<Box<dyn Send + FnOnce(&str, u16, Option<Image>)>>>,
}

ref_counter!(cef_download_image_callback_t);
impl Wrapper for DownloadImageCallbackWrapper {
    type Cef = cef_download_image_callback_t;
    fn wrap(self) -> RefCountedPtr<Self::Cef> {
        RefCountedPtr::wrap(
            cef_download_image_callback_t {
                base: unsafe { std::mem::zeroed() },
                on_download_image_finished: Some(Self::download_image_finished),
            },
            self,
        )
    }
}

impl DownloadImageCallbackWrapper {
    pub(crate) fn new(
        callback: impl Send + FnOnce(&str, u16, Option<Image>) + 'static,
    ) -> DownloadImageCallbackWrapper {
        DownloadImageCallbackWrapper {
            callback: Mutex::new(Some(Box::new(callback))),
        }
    }
}

cef_callback_impl! {
    impl for DownloadImageCallbackWrapper: cef_download_image_callback_t {
        fn download_image_finished(
            &self,
            image_url: &CefString: *const cef_string_t,
            http_status_code: std::os::raw::c_int: std::os::raw::c_int,
            image: Option<Image>: *mut cef_image_t,
        ) {
            if let Some(callback) = self.callback.lock().take() {
                callback(&String::from(image_url), http_status_code as u16, image);
            }
        }
    }
}

pub(crate) struct PDFPrintCallbackWrapper {
    callback: Mutex<Option<Box<dyn Send + FnOnce(&str, bool)>>>,
}

ref_counter!(cef_pdf_print_callback_t);
impl Wrapper for PDFPrintCallbackWrapper {
    type Cef = cef_pdf_print_callback_t;
    fn wrap(self) -> RefCountedPtr<Self::Cef> {
        RefCountedPtr::wrap(
            cef_pdf_print_callback_t {
                base: unsafe { std::mem::zeroed() },
                on_pdf_print_finished: Some(Self::pdf_print_finished),
            },
            self,
        )
    }
}

impl PDFPrintCallbackWrapper {
    pub(crate) fn new(
        callback: impl Send + FnOnce(&str, bool) + 'static,
    ) -> PDFPrintCallbackWrapper {
        PDFPrintCallbackWrapper {
            callback: Mutex::new(Some(Box::new(callback))),
        }
    }
}

cef_callback_impl! {
    impl for PDFPrintCallbackWrapper: cef_pdf_print_callback_t {
        fn pdf_print_finished(
            &self,
            path: &CefString: *const cef_string_t,
            ok: bool: std::os::raw::c_int
        ) {
            if let Some(callback) = self.callback.lock().take() {
                callback(&String::from(path), ok);
            }
        }
    }
}

pub struct NavigationEntryVisit {
    /// Current navigation entry. Do not keep a reference to this field outside of the
    /// visitor callback.
    pub entry: NavigationEntry,
    /// Whether or not this is the currently loaded navigation entry.
    pub current: bool,
    /// The 0-based index of this entry.
    pub index: usize,
    /// The total number of navigation entries.
    pub total: usize,
}

/// Callback type for `NavigationEntryVisitor`.
///
/// Returns whether or not to continue visiting more navigation entries.
pub trait NavigationEntryVisitorCallback = 'static + Send + FnMut(NavigationEntryVisit) -> bool;

ref_counted_ptr!{
    pub struct NavigationEntryVisitor(*mut cef_navigation_entry_visitor_t);
}

impl NavigationEntryVisitor {
    pub fn new<C: NavigationEntryVisitorCallback>(callback: C) -> NavigationEntryVisitor {
        unsafe{ NavigationEntryVisitor::from_ptr_unchecked(NavigationEntryVisitorWrapper::new(Box::new(callback)).wrap().into_raw()) }
    }
}

pub(crate) struct NavigationEntryVisitorWrapper {
    callback: SendProtectorMut<Box<dyn NavigationEntryVisitorCallback>>,
}

impl Wrapper for NavigationEntryVisitorWrapper {
    type Cef = cef_navigation_entry_visitor_t;
    fn wrap(self) -> RefCountedPtr<Self::Cef> {
        RefCountedPtr::wrap(
            cef_navigation_entry_visitor_t {
                base: unsafe { std::mem::zeroed() },
                visit: Some(Self::visit),
            },
            self,
        )
    }
}

impl NavigationEntryVisitorWrapper {
    pub(crate) fn new(
        callback: impl NavigationEntryVisitorCallback,
    ) -> NavigationEntryVisitorWrapper {
        NavigationEntryVisitorWrapper {
            callback: SendProtectorMut::new(Box::new(callback)),
        }
    }
}

cef_callback_impl! {
    impl for NavigationEntryVisitorWrapper: cef_navigation_entry_visitor_t {
        fn visit(
            &self,
            entry: NavigationEntry: *mut cef_navigation_entry_t,
            current: bool: std::os::raw::c_int,
            index: std::os::raw::c_int: std::os::raw::c_int,
            total: std::os::raw::c_int: std::os::raw::c_int
        ) -> std::os::raw::c_int {
            (unsafe{ &mut *self.callback.get_mut() })(NavigationEntryVisit {
                entry,
                current,
                index: index as usize,
                total: total as usize,
            } ) as _
        }
    }
}
