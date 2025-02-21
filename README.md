# tokio_telegraf

## Under development

[![tokio_telegraf crate](https://img.shields.io/crates/v/tokio_telegraf.svg)](https://crates.io/crates/tokio_telegraf)
[![tokio_telegraf crate downloads](https://img.shields.io/crates/d/tokio_telegraf)](https://crates.io/crates/tokio_telegraf)
[![tokio_telegraf documentation](https://docs.rs/tokio_telegraf/badge.svg)](https://docs.rs/tokio_telegraf)

`tokio_telegraf` is a lightweight client library for general metrics writing using Telegraf. Telegraf is a micro-service provided
by InfluxData for making metrics reporting easy for distributed services - see their [docs](https://docs.influxdata.com/telegraf/v1.13/introduction/installation/) for more information.

**This library is a fork of [maxmindlin/telegraf-rust](https://github.com/maxmindlin/telegraf-rust) to allow usage with asyncio, under the [Tokio](https://tokio.rs) ecosystem.**

This library does not provide querying or other InfluxDB client-library features. This is meant to be lightweight and simple for services to report metrics.

`tokio_telegraf` supports all socket connection types, such as UDS (unix domain socket):

- TCP (`tcp://`)
- UDP (`udp://`)
- UDS Stream (`unix://`)
- UDS Datagram (`unixgram://`)

# Differences with [maxmindlin/telegraf-rust](https://github.com/maxmindlin/telegraf-rust)
* Asyncio only operations, based on the [Tokio](https://tokio.rs) ecosystem.
* **TBD** - Fewer allocations, by reducing the usage of `Box` and by handling data-points
  lifetimes.

# Install

Add it to your Cargo.toml:

```toml
[dependencies]
tokio_telegraf = "*"
```

# How to use

Using this library assumes you have a socket listener setup in your Telegraf configuration file. An example TCP connection looks like so:

```toml
[[inputs.socket_listener]]
  service_address = "tcp://localhost:8094"
```

All usage will start by creating a socket connection via a `Client`. This supports multiple connection protocols - which one you use will be determined by how your Telegraf `input.socket_listener` configuration is setup.

Once a client is setup there are multiple different ways to write points:

## Define structs that represent metrics using the derive macro

```rust
use tokio_telegraf::*;

let mut client = Client::new("tcp://localhost:8094").await.unwrap();

#[derive(Metric)]
struct MyMetric {
    field1: i32,
    #[telegraf(tag)]
    tag1: String,
}

let point = MyMetric { field1: 1, tag1: "tag" };
client.write(&point).await;
```

By default the measurement name will be the same as the struct. You can override this via derive attributes:

```rust
use tokio_telegraf::*;

#[derive(Metric)]
#[measurement = "custom_name"]
struct MyMetric {
    field1: i32,
}
```

As with any Telegraf point, tags are optional but at least one field is required.

Timestamps are optional and can be set via the `timestamp` attribute:

```rust
use tokio_telegraf::*;

#[derive(Metric)]
struct MyMetric {
    #[telegraf(timestamp)]
    ts: u64,
    field1: i32,
}
```

## Use the `point` macro to do ad-hoc metrics

```rust
use tokio_telegraf::*;

let mut client = Client::new("tcp://localhost:8094").await.unwrap();

let p = point!("measurement", ("tag1", "tag1Val"), ("field1", "val") ("field2", 10); 100);
client.write_point(&p).await;
```

The macro syntax is the following format:

```
(<measurement>, [(<tagName>, <tagVal>)], [(<fieldName>, <fieldVal>)]; <timestamp>)
```

Measurement name, tag set, and field set are comma separated. Tag and field tuples are space separated. Timestamp is semicolon separated. The tag set and timestamp are optional.

## Manual `Point` initialization

```rust
use tokio_telegraf::{Client, Point};

let c = Client::new("tcp://localhost:8094").await.unwrap();

let p = Point::new(
    String::from("measurement"),
    vec![
        (String::from("tag1"), String::from("tag1value"))
    ],
    vec![
        (String::from("field1"), Box::new(10)),
        (String::from("field2"), Box::new(20.5)),
        (String::from("field3"), Box::new("anything!"))
    ],
    Some(100),
);

c.write_point(p).await
```

### Field Data

Any attribute that will be the value of a field must implement the `IntoFieldData` trait provided by this library.

```rust
pub trait IntoFieldData {
    fn field_data(&self) -> FieldData;
}
```

Out of the box implementations are provided for many common data types, but manual implementation is possible for other data types.

### Timestamps

Timestamps are optional. If not present, the Telegraf daemon will set the timestamp using the current time.
Timestamps are specified in nanosecond-precision Unix time, therefore `u64` must implement the `From<T>` trait for the field type, if the implementation is not already present:

```rust
use tokio_telegraf::*;

#[derive(Copy, Clone)]
struct MyType {
    // ...
}

impl From<MyType> for u64 {
    fn from(my_type: MyType) -> Self {
        todo!()
    }
}

#[derive(Metric)]
struct MyMetric {
    #[telegraf(timestamp)]
    ts: MyType,
    field1: i32,
}

```

More information about timestamps can be found [here](https://docs.influxdata.com/influxdb/v1.8/write_protocols/line_protocol_tutorial/#timestamp).
