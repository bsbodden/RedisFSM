#[macro_use] extern crate redis_module;

use redis_module::{Context, RedisError, RedisResult, RedisString, NextArg};

fn fsm_create(_: &Context, args: Vec<RedisString>) -> RedisResult {
  let args = args.into_iter().skip(1);
  let args_card = args.len();

  if args_card > 1 {
    return Err(RedisError::WrongArity);
  }

  let src = args.into_iter().next_string()?;
  let greet = format!("👋 Hello {}", src);
  let response = Vec::from(greet);

  return Ok(response.into());
}

//////////////////////////////////////////////////////

redis_module! {
  name: "fsm",
  version: 1,
}
