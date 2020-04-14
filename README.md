# Cloudflare Internship Application: Systems

## Ping application
The following application can be found in the git repo: https://github.com/arjo129/internship-application-systems

The application implements a ping client in rust. 

To build use cargo.
```
cargo build
```

To execute one needs to give the executable to have permission to create a raw socket on linux this is done by granting the capability `CAP_NET_RAW` to this executable.
Alternatively you may run the executable as root.


## Executable usage

Basic usage:
```
mping <hostname or ip>
``` 
For example:

```
mping www.google.com
```
```
You can also set TTL like so:
```
mping <hostname or ip> <TTL>
```
For example:
```
mping www.google.com 2
```
If there is a "time out", the system will report it.
##Supported Features

* Basic ipv4 Ping functionality
* Set time to live
