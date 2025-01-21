use std::{sync::LazyLock, time::Duration};

use futures::future::LocalBoxFuture;
use reqwest::header::{HeaderMap, ACCEPT, USER_AGENT};
use rquickjs::{
    function::Async as AsyncJsClosure,
    prelude::{Func, Opt},
    Array, AsyncContext, AsyncRuntime, Coerced, Ctx, FromJs, IntoJs, IteratorJs, Object,
    String as JsString, Type, Value,
};
use tokio::task::LocalSet;
use twixel_core::irc_message::{tags::OwnedTag, PrivMsg};

use crate::{
    bot::BotCommand,
    command::{CommandContext, CommandHandler},
    util::sanitize_output,
};

pub struct EvalHandler {
    cx_sender: tokio::sync::mpsc::Sender<CommandContext<BotCommand>>,
}

impl EvalHandler {
    pub fn new() -> Self {
        let tx = eval_thread();
        Self { cx_sender: tx }
    }
}

impl CommandHandler for EvalHandler {
    fn handle(
        &self,
        cx: CommandContext<BotCommand>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + Sync>> {
        let tx = self.cx_sender.clone();
        Box::pin(async move {
            tx.send(cx).await.unwrap();
        })
    }
}

const MAX_EVAL_DURATION: Duration = Duration::from_secs(5);

fn eval_thread() -> tokio::sync::mpsc::Sender<CommandContext<BotCommand>> {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<CommandContext<BotCommand>>(16);

    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .thread_name("quickjs eval")
            .build()
            .unwrap()
            .block_on(async move {
                let local_set = LocalSet::new();

                local_set.spawn_local(async move {
                    loop {
                        let cx = rx.recv().await.unwrap();

                        tokio::task::spawn_local(async move {
                            let rt = AsyncRuntime::new().unwrap();

                            rt.set_memory_limit(50_000_000).await;
                            let start_time = std::time::Instant::now();

                            rt.set_interrupt_handler(Some(Box::new(move || {
                                start_time.elapsed().as_secs() > 5
                            })))
                            .await;

                            let context = AsyncContext::full(&rt).await.unwrap();

                            tokio::task::spawn_local(async move {
                                log::debug!("driving new quickjs runtime");
                                rt.drive().await;
                            });

                            log::debug!("received js task");

                            context
                                .with(|ctx| {
                                    let cloned_ctx = ctx.clone();
                                    ctx.spawn(async move {
                                        let _ = tokio::time::timeout(
                                            MAX_EVAL_DURATION,
                                            eval(cloned_ctx, cx),
                                        )
                                        .await;
                                    });
                                })
                                .await;
                        });
                    }
                });

                local_set.await;
            });
    });

    tx
}

fn rquickjs_err_to_pretty(err: rquickjs::Error, ctx: &Ctx) -> String {
    if !err.is_exception() {
        return err.to_string();
    }
    ctx.catch()
        .into_exception()
        .map(|v| {
            let name = v
                .get_prototype()
                .and_then(|prot| prot.get::<_, Object>("constructor").ok())
                .and_then(|cons| cons.get::<_, Coerced<String>>("name").ok())
                .map(|c| c.0)
                .unwrap();
            format!(
                "[{name}] thrown: {:?}",
                v.message().as_deref().unwrap_or("{no message set}")
            )
        })
        .unwrap_or_else(|| err.to_string())
}

fn repl_print_value(val: Value<'_>) -> LocalBoxFuture<'_, String> {
    Box::pin(async {
        let ctx = val.ctx().clone();

        match val.type_of() {
            Type::Undefined
            | Type::Null
            | Type::Bool
            | Type::Int
            | Type::Float
            | Type::Function
            | Type::BigInt
            | Type::Constructor
            | Type::Symbol
            | Type::Uninitialized => Coerced::<String>::from_js(&ctx, val).map(|i| i.0).unwrap(),
            Type::String => val
                .as_string()
                .map(|i| {
                    format!(
                        "\"{}\"",
                        i.to_string().unwrap_or("invalid UTF-8 string".into())
                    )
                })
                .unwrap(),
            Type::Array | Type::Exception | Type::Object | Type::Module | Type::Unknown => ctx
                .json_stringify(val)
                .and_then(|i| i.map(|s| s.to_string()).unwrap())
                .expect("aaaa"),
            Type::Promise => match val.into_promise().unwrap().into_future().await {
                Ok(v) => repl_print_value(v).await,
                Err(e) => rquickjs_err_to_pretty(e, &ctx),
            },
        }
    })
}

async fn eval<'js>(ctx: Ctx<'js>, cx: CommandContext<BotCommand>) {
    let source_channel: String = cx.msg.get_param(0).unwrap().split_at(1).1.into();
    let Some(code) = cx
        .msg
        .get_param(1)
        .and_then(|s| s.split_once(' ').map(|s| s.1))
        .map(|s| s.to_string())
    else {
        return;
    };

    let globals = ctx.globals();

    globals.remove("eval").unwrap();

    let msg = PrivMsg::from_any(cx.msg);

    let js_ctx = msg.map(|m| JsContext::new(m));

    globals.set("context", js_ctx).unwrap();

    let tx_clone = cx.bot_tx.clone();
    let src_clone = source_channel.clone();

    globals
        .set(
            "send",
            Func::new(AsyncJsClosure(move |msg: String| {
                let src = src_clone.clone();
                let tx = tx_clone.clone();
                async move {
                    tx.send(BotCommand::SendMessage {
                        channel_login: src,
                        message: format!("🤖 {msg}"),
                        reply_id: None,
                    })
                    .await
                    .is_ok()
                }
            })),
        )
        .unwrap();

    globals.set("fetch", js_fetch).unwrap();

    let mut out: String = match ctx.eval_promise(code) {
        Ok(p) => {
            let val = p.into_future::<Value>().await;

            match val.and_then(|val| val.as_object().unwrap().get::<_, Value>("value")) {
                Ok(val) => repl_print_value(val).await,
                Err(e) => rquickjs_err_to_pretty(e, &ctx),
            }
        }
        Err(e) => rquickjs_err_to_pretty(e, &ctx),
    };

    sanitize_output(&mut out);

    cx.bot_tx
        .send(BotCommand::SendMessage {
            channel_login: source_channel.clone(),
            message: out,
            reply_id: None,
        })
        .await
        .unwrap();
}

struct JsContext {
    msg: String,
    user_login: Option<String>,
    user_id: Option<String>,
}

impl<'js> IntoJs<'js> for JsContext {
    fn into_js(self, ctx: &Ctx<'js>) -> rquickjs::Result<Value<'js>> {
        let out = Object::new(ctx.clone())?;

        out.set("msg", self.msg)?;
        out.set("user_login", self.user_login.as_deref())?;
        out.set("user_id", self.user_id.as_deref())?;

        Ok(out.into_value())
    }
}

impl JsContext {
    fn new(msg: PrivMsg) -> Self {
        Self {
            msg: msg.message_text().to_owned(),
            user_login: msg.sender_login().map(|s| s.to_owned()),
            user_id: msg.sender_id().map(|s| s.to_owned()),
        }
    }
}
#[derive(smart_default::SmartDefault)]
struct JsRequestInit {
    body: Option<String>,
    #[default(reqwest::Method::GET)]
    method: reqwest::Method,
}

impl<'js> FromJs<'js> for JsRequestInit {
    fn from_js(ctx: &rquickjs::Ctx<'js>, value: Value<'js>) -> rquickjs::Result<Self> {
        let type_name = value.type_name();
        let obj = value
            .try_into_object()
            .map_err(|_| rquickjs::Error::new_from_js(type_name, "object"))?;

        let body = String::from_js(ctx, obj.get("body")?).ok();
        let method = String::from_js(ctx, obj.get("method")?)?
            .parse()
            .unwrap_or_default();

        Ok(Self { body, method })
    }
}

static JS_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    let mut default_headers = HeaderMap::new();
    default_headers.insert(
        USER_AGENT,
        format!("twixel-rs/{}", env!("CARGO_PKG_VERSION"))
            .parse()
            .unwrap(),
    );
    default_headers.insert(ACCEPT, "application/json, text/plain".parse().unwrap());

    reqwest::ClientBuilder::new()
        .default_headers(default_headers)
        .https_only(true)
        .user_agent(format!("twixel-rs/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .unwrap()
});

#[derive(Debug, thiserror::Error)]
enum FetchError {
    #[error(transparent)]
    RQuickJsError(#[from] rquickjs::Error),
    #[error(transparent)]
    RequestError(#[from] reqwest::Error),
}

impl<'js> IntoJs<'js> for FetchError {
    fn into_js(self, ctx: &Ctx<'js>) -> rquickjs::Result<Value<'js>> {
        rquickjs::Exception::from_message(ctx.clone(), &self.to_string()).map(|v| v.into_value())
    }
}

struct ValueJs(serde_json::Value);

impl<'js> IntoJs<'js> for ValueJs {
    fn into_js(self, ctx: &Ctx<'js>) -> rquickjs::Result<Value<'js>> {
        match self.0 {
            serde_json::Value::Null => Ok(Value::new_null(ctx.clone())),
            serde_json::Value::Bool(val) => val.into_js(ctx),
            serde_json::Value::Number(number) => number.as_f64().unwrap().into_js(ctx),
            serde_json::Value::String(s) => s.into_js(ctx),
            serde_json::Value::Array(vec) => vec
                .into_iter()
                .map(ValueJs)
                .collect_js::<Array<'js>>(ctx)
                .map(|a| a.into_value()),
            serde_json::Value::Object(map) => map
                .into_iter()
                .map(|(k, v)| (k, ValueJs(v)))
                .collect_js::<Object<'js>>(ctx)
                .map(|o| o.into_value()),
        }
    }
}

async fn fetch_inner(url: JsString<'_>, opts: JsRequestInit) -> Result<Object<'_>, FetchError> {
    let ctx = url.ctx();

    let url = url.to_string()?;

    let resp = JS_CLIENT
        .execute(JS_CLIENT.request(opts.method, url).build()?)
        .await?;
    let status = resp.status();
    let resp_text = resp.text().await?;
    match serde_json::from_str::<serde_json::Value>(&resp_text) {
        Ok(o) => {
            let out = rquickjs::Object::new(ctx.clone())?;
            out.set("body", ValueJs(o))?;
            out.set("status", status.as_u16())?;
            Ok(out)
        }
        Err(_) => {
            let out = rquickjs::Object::new(ctx.clone())?;
            out.set("body", resp_text)?;
            out.set("status", status.as_u16())?;
            Ok(out)
        }
    }
}

#[rquickjs::function]
async fn fetch(url: JsString<'_>, opts: Opt<JsRequestInit>) -> Result<Object<'_>, rquickjs::Error> {
    let ctx = url.ctx();

    let opts = opts.0.unwrap_or_default();

    match fetch_inner(url.clone(), opts).await {
        Ok(ok) => Ok(ok),
        Err(FetchError::RQuickJsError(e)) => Err(e),
        Err(FetchError::RequestError(e)) => Err(
            ctx.throw(rquickjs::Exception::from_message(ctx.clone(), &e.to_string())?.into_value())
        ),
    }
}
