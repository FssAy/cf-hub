# Cloudflare HUB
Small executable to run multiple HTTP servers on the same machine and be able to connect to each using a single port via Cloudflare.

### Motivation
Cloudflare doesn't allow to specify origin server's port on a free tier, so when you run 2 HTTP servers.
First on port 1234 and the second one on 5678 in order to connect to them, you have to specify the port in the URL.
Just as follows: `server1.com:1234` and `server2.org:5678`. 

### Solution
I think there might be a way to do that with Cloudflare redirects, but it didn't work for me.

This application will intercept every request intended for each defined host and will redirect it to a specified IP.

### How to use
On the machine:
  1. Run the `cf-hub` to generate a config `cf-hub-cfg.json`.
  2. In the config file:
     - Set `addr_server` to the address on which the CF-HUB should operate. 
     In most cases it should be your machine's WAN IP and port 443.
     - In `hosts` specify your hosts and servers where the JSON value 
     is an IP and port of the application serving the website. For example: `"server1.com": "127.0.0.1:1234"`.
  3. Run your servers.
  4. Run `cf-hub`.

### Disclaimer
1. This is a simple program which shouldn't be used with high demanding platforms.
It's intended for small hobby projects where the owner doesn't have enough funds to 
use different machines for each server.


2. It doesn't support WebSockets.


3. It's intended for Cloudflare domains, it won't work separately.


3. There might be some unforeseen issues due to the nature of HTTP.

### ToDo:
- [ ] Better error handling.
- [ ] Support WebSockets.
- [x] Fix issues with TLS.
