#[macro_use] extern crate redis_module;
#[macro_use] extern crate guard;

use redis_module::{Context, RedisResult, RedisString, RedisValue, RedisError, NextArg, NotifyEvent};
use serde::{Deserialize, Serialize};
use redis_module::native_types::RedisType;
use redis_module::raw::RedisModuleTypeMethods;
use std::os::raw::c_void;

//////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Debug)]
struct Event {
  name: String,
  from: Vec<String>,
  to: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct StateMachine {
  name: String,
  prefix: String,
  field: String,
  states: Vec<String>,
  events: Vec<Event>,
}

unsafe extern "C" fn free(value: *mut c_void) {
  Box::from_raw(value.cast::<StateMachine>());
}

//////////////////////////////////////////////////////

pub const REDIS_FSM_TYPE_NAME: &str = "Redis-FSM";
pub const REDIS_FSM_TYPE_VERSION: i32 = 1;

pub static REDIS_FSM_TYPE: RedisType = RedisType::new(
  REDIS_FSM_TYPE_NAME,
  REDIS_FSM_TYPE_VERSION,
  RedisModuleTypeMethods {
    version: redis_module::TYPE_METHOD_VERSION,
    rdb_load: None,
    rdb_save: None,
    aof_rewrite: None,
    free: Some(free),
    mem_usage: None,
    digest: None,
    aux_load: None,
    aux_save: None,
    aux_save_triggers: 0,
    free_effort: None,
    unlink: None,
    copy: None,
    defrag: None,
  },
);

//////////////////////////////////////////////////////
const REDIS_FSM_HASH_NAME: &str = "Redis-FSM-Hash";

fn fsm_create(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
  let mut args = args.into_iter().skip(1);
  let fsm_json = args.next_arg()?;
  let fsm: StateMachine = serde_json::from_str(&fsm_json.to_string())?;
  let key = RedisString::create(ctx.ctx, &fsm.name.to_string());
  let redis_key = ctx.open_key_writable(&key);
  let prefix: &str = &fsm.prefix.clone();

  guard!(let Ok(_) = redis_key.set_value(&REDIS_FSM_TYPE, fsm) else { return Err(RedisError::Str("ERR could not persist state machine")) });

  return ctx.call("HSET", &[&REDIS_FSM_HASH_NAME, prefix, &key.to_string()]);
}

fn fsm_info(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
  let mut args = args.into_iter().skip(1);
  let key = args.next_arg()?;
  let redis_key = ctx.open_key(&key);

  guard!(let Ok(Some(fsm)) = redis_key.get_value::<StateMachine>(&REDIS_FSM_TYPE) else { return Err(RedisError::Str("ERR key not found")) });

  let json = serde_json::to_string(fsm)?;
  return Ok(RedisValue::SimpleString(json));
}

//////////////////////////////////////////////////////

fn on_event(ctx: &Context, event_type: NotifyEvent, event: &str, key: &str) {
  let msg = format!(
      "Received event: {:?} on key: {} via event: {}",
      event_type, key, event
  );
  ctx.log_notice(msg.as_str());
  // split the key to get the prefix
  let key_parts: Vec<&str> = key.split(':').collect();
  guard!(let Some(prefix) = key_parts.into_iter().nth(0) else { return });
  let key_prefix = &format!("{}:", prefix);
  // need to find the correct fsm for the key prefix
}

//////////////////////////////////////////////////////

redis_module! {
  name: "fsm",
  version: 1,
  data_types: [
    REDIS_FSM_TYPE
  ],
  commands: [
    ["fsm.create", fsm_create, "write", 1, 1, 1],
    ["fsm.info", fsm_info, "readonly", 1, 1, 1],
  ],
  event_handlers: [
    [@HASH: on_event],
  ]
}
