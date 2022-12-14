#[macro_use] extern crate redis_module;
#[macro_use] extern crate guard;

use redis_module::{Context, RedisResult, RedisString, RedisValue, RedisError, NextArg, NotifyEvent};
use serde::{Deserialize, Serialize};
use redis_module::native_types::RedisType;
use redis_module::raw::RedisModuleTypeMethods;
use std::os::raw::{c_int, c_void};
use redis_module::raw;
use std::{ffi::CString, ptr::null_mut};

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

impl StateMachine {
  fn initial_state(&self) -> Option<&String> {
    return self.states.first();
  }

  fn allowed(&self, ctx: &Context, key: RedisString, fsm_event: RedisString) -> Option<&Event> {
    // 1 - Load the Hash state field with HGET
    guard!(let Ok(response) = ctx.call("HGET", &[&key.to_string(), &self.field]) else { return None });
    // 2 - Find the event struct by name - in self.states
    guard!(let RedisValue::SimpleString(current_state) = response else { return None });
    guard!(let Some(event) = self.events.iter().find(|&e| e.name == fsm_event.to_string()) else { return None });
    // 3 - If current state is in the "from" field, the transition is allowed, return the event to the caller
    if event.from.iter().any(|from| from == &current_state) {
      Some(event)
    } else {
      None
    }
  }

  fn trigger(&self, ctx: &Context, key: RedisString, fsm_event: RedisString) -> bool {
    let hash_key = &key.to_string();
    guard!(let Some(event) = self.allowed(ctx, key, fsm_event) else { return false });
    let hset_result = ctx.call("HSET", &[hash_key, &self.field, &event.to]);
    return matches!(hset_result, Ok(_));
  }
}

unsafe extern "C" fn free(value: *mut c_void) {
  Box::from_raw(value.cast::<StateMachine>());
}

extern "C" fn rdb_load(rdb: *mut raw::RedisModuleIO, _encver: c_int) -> *mut c_void {
  guard!(let Ok(data) = raw::load_string(rdb) else { return null_mut() });
  let json_string = data.to_string();
  let fsm: StateMachine = serde_json::from_str(&json_string.to_string()).unwrap();
  return Box::into_raw(Box::new(fsm)).cast::<c_void>();
}

unsafe extern "C" fn rdb_save(rdb: *mut raw::RedisModuleIO, value: *mut c_void) {
  let mut out = serde_json::Serializer::new(Vec::new());
  let fsm = &*value.cast::<StateMachine>();
  fsm.serialize(&mut out).unwrap();
  let json = String::from_utf8(out.into_inner()).unwrap();
  let cjson = CString::new(json).unwrap();
  raw::save_string(rdb, cjson.to_str().unwrap());
}

//////////////////////////////////////////////////////

pub const REDIS_FSM_TYPE_NAME: &str = "Redis-FSM";
pub const REDIS_FSM_TYPE_VERSION: i32 = 1;

pub static REDIS_FSM_TYPE: RedisType = RedisType::new(
  REDIS_FSM_TYPE_NAME,
  REDIS_FSM_TYPE_VERSION,
  RedisModuleTypeMethods {
    version: redis_module::TYPE_METHOD_VERSION,
    rdb_load: Some(rdb_load),
    rdb_save: Some(rdb_save),
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

fn fsm_allowed(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
  let mut args = args.into_iter().skip(1);
  let fsm_key = args.next_arg()?;
  let redis_key = ctx.open_key(&fsm_key);
  let hash_key = args.next_arg()?;
  let event = args.next_arg()?;

  guard!(let Ok(Some(fsm)) = redis_key.get_value::<StateMachine>(&REDIS_FSM_TYPE) else { return Err(RedisError::Str("ERR key not found")) });

  if let Some(_event) = fsm.allowed(ctx, hash_key, event) {
    return Ok(RedisValue::Integer(true as i64));
  } else {
    return Ok(RedisValue::Integer(false as i64));
  }
}

fn fsm_trigger(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
  let mut args = args.into_iter().skip(1);
  let fsm_key = args.next_arg()?;
  let redis_key = ctx.open_key(&fsm_key);
  let hash_key = args.next_arg()?;
  let event = args.next_arg()?;

  guard!(let Ok(Some(fsm)) = redis_key.get_value::<StateMachine>(&REDIS_FSM_TYPE) else { return Err(RedisError::Str("ERR key not found")) });

  let allowed = fsm.trigger(ctx, hash_key, event);
  return Ok(RedisValue::Integer(allowed as i64));
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
  guard!(let Ok(RedisValue::SimpleString(fsm_key)) = ctx.call("HGET", &[&REDIS_FSM_HASH_NAME, key_prefix]) else { return });
  let key_name = RedisString::create(ctx.ctx, &fsm_key);
  let redis_key = ctx.open_key(&key_name);
  guard!(let Ok(Some(fsm)) = redis_key.get_value::<StateMachine>(&REDIS_FSM_TYPE) else { return });
  if let Ok(RedisValue::Null) = ctx.call("HGET", &[&key.to_string(), &fsm.field]) {
    // set the initial state of the hash if the field is null
    guard!(let Some(initial_state) = fsm.initial_state() else { return });
    _ = ctx.call("HSET", &[&key.to_string(), &fsm.field, &initial_state]);
  }
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
    ["fsm.allowed", fsm_allowed, "readonly", 1, 2, 1],
    ["fsm.trigger", fsm_trigger, "write", 1, 2, 1],
  ],
  event_handlers: [
    [@HASH: on_event],
  ]
}
