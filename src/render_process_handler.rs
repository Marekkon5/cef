use cef_sys::{
    cef_browser_t, cef_dictionary_value_t, cef_domnode_t, cef_frame_t,
    cef_list_value_t, cef_load_handler_t, cef_process_id_t, cef_process_message_t,
    cef_render_process_handler_t, cef_v8context_t, cef_v8exception_t, cef_v8stack_trace_t,
};
use std::{sync::Arc};
use parking_lot::Mutex;
use crate::{
    browser::Browser,
    client::Client,
    dom::DOMNode,
    frame::Frame,
    load_handler::{LoadHandler, LoadHandlerWrapper},
    process::{ProcessId, ProcessMessage},
    refcounted::{RefCountedPtr, RefCountedPtrCache, Wrapper},
    v8context::{V8Context, V8Exception, V8StackTrace},
    values::{DictionaryValue, ListValue},
};

/// Trait used to implement render process callbacks. The functions of this
/// trait will be called on the render process main thread ([ProcessId::Renderer])
/// unless otherwise indicated.
pub trait RenderProcessHandler<C: Client>: Send + Sync {
    /// Called after the render process main thread has been created. `extra_info`
    /// is originating from
    /// [BrowserProcessHandler::on_render_process_thread_created].
    fn on_render_thread_created(&self, extra_info: &ListValue) {}
    /// Called after WebKit has been initialized.
    fn on_web_kit_initialized(&self) {}
    /// Called after a browser has been created. When browsing cross-origin a new
    /// browser will be created before the old browser with the same identifier is
    /// destroyed. |extra_info| is originating from
    /// [BrowserHost::create_browser()],
    /// [BrowserHost::create_browser_sync()],
    /// [LifeSpanHandler::on_before_popup()] or [BrowserView::create()].
    fn on_browser_created(&self, browser: Browser<C>, extra_info: &DictionaryValue) {}
    /// Called before a browser is destroyed.
    fn on_browser_destroyed(&self, browser: Browser<C>) {}
    /// Return the handler for browser load status events.
    fn get_load_handler(&self) -> Option<Arc<dyn LoadHandler<C> + 'static>> {
        None
    }
    /// Called immediately after the V8 context for a frame has been created. To
    /// retrieve the JavaScript 'window' object use the
    /// [V8Context::get_global()] function. V8 handles can only be accessed
    /// from the thread on which they are created. A task runner for posting tasks
    /// on the associated thread can be retrieved via the
    /// [V8Context::get_task_runner()] function.
    fn on_context_created(&self, browser: Browser<C>, frame: Frame<C>, context: V8Context<C>) {}
    /// Called immediately before the V8 context for a frame is released.
    fn on_context_released(&self, browser: Browser<C>, frame: Frame<C>, context: V8Context<C>) {}
    /// Called for global uncaught exceptions in a frame. Execution of this
    /// callback is disabled by default. To enable set
    /// [CefSettings.uncaught_exception_stack_size] > 0.
    fn on_uncaught_exception(
        &self,
        browser: Browser<C>,
        frame: Frame<C>,
        context: V8Context<C>,
        exception: V8Exception,
        stack_trace: V8StackTrace,
    ) {
    }
    /// Called when a new node in the the browser gets focus. The `node` value may
    /// be None if no specific node has gained focus. The node object passed to
    /// this function represents a snapshot of the DOM at the time this function is
    /// executed.
    fn on_focused_node_changed(&self, browser: Browser<C>, frame: Frame<C>, node: Option<&DOMNode>) {}
    /// Called when a new message is received from a different process. Return true
    /// if the message was handled or false otherwise.
    fn on_process_message_received(
        &self,
        browser: Browser<C>,
        frame: Frame<C>,
        source_process: ProcessId,
        message: &ProcessMessage,
    ) -> bool {
        false
    }
}

pub(crate) struct RenderProcessHandlerWrapper<C: Client> {
    delegate: Arc<dyn RenderProcessHandler<C>>,
    load_handler: Mutex<Option<RefCountedPtrCache<cef_load_handler_t>>>,
}

unsafe impl<C: Client> Send for RenderProcessHandlerWrapper<C> {}
unsafe impl<C: Client> Sync for RenderProcessHandlerWrapper<C> {}

impl<C: Client> RenderProcessHandlerWrapper<C> {
    pub(crate) fn new(
        delegate: Arc<dyn RenderProcessHandler<C>>,
    ) -> Self {
        Self {
            delegate,
            load_handler: Mutex::new(None),
        }
    }
}

impl<C: Client> std::borrow::Borrow<Arc<dyn RenderProcessHandler<C>>> for RenderProcessHandlerWrapper<C> {
    fn borrow(&self) -> &Arc<dyn RenderProcessHandler<C>> {
        &self.delegate
    }
}

impl<C: Client> Wrapper for RenderProcessHandlerWrapper<C> {
    type Cef = cef_render_process_handler_t;
    type Inner = dyn RenderProcessHandler<C>;
    fn wrap(self) -> RefCountedPtr<Self::Cef> {
        RefCountedPtr::wrap(
            cef_render_process_handler_t {
                base: unsafe { std::mem::zeroed() },
                on_render_thread_created: Some(Self::render_thread_created),
                on_web_kit_initialized: Some(Self::web_kit_initialized),
                on_browser_created: Some(Self::browser_created),
                on_browser_destroyed: Some(Self::browser_destroyed),
                get_load_handler: Some(Self::get_load_handler),
                on_context_created: Some(Self::context_created),
                on_context_released: Some(Self::context_released),
                on_uncaught_exception: Some(Self::uncaught_exception),
                on_focused_node_changed: Some(Self::focused_node_changed),
                on_process_message_received: Some(Self::process_message_received),
            },
            self,
        )
    }
}

cef_callback_impl!{
    impl<C: Client> for RenderProcessHandlerWrapper<C>: cef_render_process_handler_t {
        fn render_thread_created<C: Client>(
            &self,
            extra_info: ListValue: *mut cef_list_value_t,
        ) {
            self.delegate
                .on_render_thread_created(&extra_info);
        }

        fn web_kit_initialized<C: Client>(&self) {
            self.delegate.on_web_kit_initialized();
        }

        fn browser_created<C: Client>(
            &self,
            browser: Browser<C>: *mut cef_browser_t,
            extra_info: DictionaryValue: *mut cef_dictionary_value_t,
        ) {
            self
                .delegate
                .on_browser_created(browser, &extra_info);
        }

        fn browser_destroyed<C: Client>(
            &self,
            browser: Browser<C>: *mut cef_browser_t,
        ) {
            self
                .delegate
                .on_browser_destroyed(browser);
        }

        fn get_load_handler<C: Client>(
            &self,
        ) -> *mut cef_load_handler_t {
            if let Some(handler) = self.delegate.get_load_handler() {
                self.load_handler
                    .lock()
                    .get_or_insert_with(|| RefCountedPtrCache::new(LoadHandlerWrapper::new(handler.clone())))
                    .get_ptr_or_rewrap(LoadHandlerWrapper::new(handler))
                    .into_raw()
            } else {
                *self.load_handler.lock() = None;
                std::ptr::null_mut()
            }
        }

        fn context_created<C: Client>(
            &self,
            browser: Browser<C>: *mut cef_browser_t,
            frame: Frame<C>: *mut cef_frame_t,
            context: V8Context<C>: *mut cef_v8context_t,
        ) {
            self.delegate.on_context_created(
                browser,
                frame,
                context,
            );
        }

        fn context_released<C: Client>(
            &self,
            browser: Browser<C>: *mut cef_browser_t,
            frame: Frame<C>: *mut cef_frame_t,
            context: V8Context<C>: *mut cef_v8context_t,
        ) {
            self.delegate.on_context_created(
                browser,
                frame,
                context,
            );
        }

        fn uncaught_exception<C: Client>(
            &self,
            browser: Browser<C>: *mut cef_browser_t,
            frame: Frame<C>: *mut cef_frame_t,
            context: V8Context<C>: *mut cef_v8context_t,
            exception: V8Exception: *mut cef_v8exception_t,
            stack_trace: V8StackTrace: *mut cef_v8stack_trace_t,
        ) {
            self.delegate.on_uncaught_exception(
                browser,
                frame,
                context,
                exception,
                stack_trace,
            );
        }

        fn focused_node_changed<C: Client>(
            &self,
            browser: Browser<C>: *mut cef_browser_t,
            frame: Frame<C>: *mut cef_frame_t,
            node: Option<DOMNode>: *mut cef_domnode_t,
        ) {
            self.delegate.on_focused_node_changed(
                browser,
                frame,
                node.as_ref()
            )
        }

        fn process_message_received<C: Client>(
            &self,
            browser: Browser<C>: *mut cef_browser_t,
            frame: Frame<C>: *mut cef_frame_t,
            source_process: ProcessId: cef_process_id_t::Type,
            message: ProcessMessage: *mut cef_process_message_t,
        ) -> std::os::raw::c_int {
            self.delegate.on_process_message_received(
                browser,
                frame,
                source_process,
                &message
            ) as std::os::raw::c_int
        }
    }
}
