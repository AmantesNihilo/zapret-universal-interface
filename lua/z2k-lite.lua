-- z2k-lite.lua
--
-- Small experimental helpers inspired by the external z2k scripts.
-- Keep this file intentionally conservative: no persistent state files,
-- no autocircular wrappers, and no dynamic strategy database.

local z2k_wrapped = rawget(_G, "__z2k_lite_wrapped") or {}
_G.__z2k_lite_wrapped = z2k_wrapped

local function z2k_log(message)
	if type(DLOG) == "function" then
		DLOG("z2k_lite: "..message)
	end
end

local function z2k_num(value, fallback)
	local n = tonumber(value)
	if n == nil then return fallback end
	return n
end

local function z2k_clamp(value, lo, hi, fallback)
	local n = z2k_num(value, fallback)
	if n < lo then return lo end
	if n > hi then return hi end
	return n
end

local function z2k_arg_enabled(value)
	return value ~= nil and value ~= false and value ~= "0" and value ~= "false" and value ~= "off"
end

local function z2k_range_pos(value)
	if type(value) ~= "string" then return nil end
	local a, b = value:match("^(%d+)%-(%d+)$")
	if not a then return nil end
	a, b = tonumber(a), tonumber(b)
	if not a or not b or a > b then return nil end
	return a, b
end

local function z2k_range_signed(value)
	if type(value) ~= "string" then return nil end
	local a, b = value:match("^(%-?%d+)%-(%-?%d+)$")
	if not a then return nil end
	a, b = tonumber(a), tonumber(b)
	if not a or not b or a > b then return nil end
	return a, b
end

local function z2k_range_cache(desync)
	local state = desync and desync.track and desync.track.lua_state
	if type(state) ~= "table" then return nil end
	if type(state.__z2k_lite_ranges) ~= "table" then
		state.__z2k_lite_ranges = {}
	end
	return state.__z2k_lite_ranges
end

local function z2k_resolve_range(desync, key, parser)
	if not desync or not desync.arg then return end
	local value = desync.arg[key]
	local lo, hi = parser(value)
	if not lo then return end

	local resolved
	local cache = z2k_range_cache(desync)
	local instance = tostring(desync.func_instance or "?")
	local cache_key = instance..":"..key
	if cache then
		resolved = cache[cache_key]
		if not resolved then
			resolved = math.random(lo, hi)
			cache[cache_key] = resolved
			z2k_log(key.."="..value.." -> "..resolved)
		end
	else
		resolved = math.floor((lo + hi) / 2)
	end
	desync.arg[key] = tostring(resolved)
end

local function z2k_resolve_ranges(desync)
	z2k_resolve_range(desync, "repeats", z2k_range_pos)
	z2k_resolve_range(desync, "seqovl", z2k_range_pos)
	z2k_resolve_range(desync, "tcp_seq", z2k_range_signed)
	z2k_resolve_range(desync, "tcp_ts", z2k_range_signed)
end

local function z2k_wrap_range_args(name)
	if z2k_wrapped[name] then return end
	local original = _G[name]
	if type(original) ~= "function" then
		z2k_log("cannot wrap "..name)
		return
	end
	z2k_wrapped[name] = original
	_G[name] = function(ctx, desync)
		z2k_resolve_ranges(desync)
		return original(ctx, desync)
	end
end

for _, name in ipairs({
	"fake",
	"multisplit",
	"multidisorder",
	"fakedsplit",
	"fakeddisorder",
	"hostfakesplit",
	"syndata",
}) do
	z2k_wrap_range_args(name)
end

-- Custom fooling hook. Use as: fool=z2k_dynamic_ttl.
function z2k_dynamic_ttl(dis, options)
	if dis and dis.ip and dis.ip.ip_ttl and dis.ip.ip_ttl > 1 then
		if not (options and (options.ip_ttl or options.ip_autottl)) then
			local prev = dis.ip.ip_ttl
			dis.ip.ip_ttl = prev - 1
			z2k_log("dynamic_ttl ipv4 "..prev.." -> "..dis.ip.ip_ttl)
		end
	end
	if dis and dis.ip6 and dis.ip6.ip6_hlim and dis.ip6.ip6_hlim > 1 then
		if not (options and (options.ip6_ttl or options.ip6_autottl)) then
			local prev = dis.ip6.ip6_hlim
			dis.ip6.ip6_hlim = prev - 1
			z2k_log("dynamic_ttl ipv6 "..prev.." -> "..dis.ip6.ip6_hlim)
		end
	end
end

-- Conservative HTTP header modifier. It does not raw-send new packets; it
-- only changes the outgoing HTTP request and lets winws2 continue normally.
function z2k_http_soft(ctx, desync)
	if not desync.dis.tcp then
		if not desync.dis.icmp then instance_cutoff_shim(ctx, desync) end
		return
	end
	direction_cutoff_opposite(ctx, desync)
	if not direction_check(desync) or desync.l7payload ~= "http_req" then return end

	local payload = desync.dis.payload
	if not payload or #payload == 0 then return end

	local prefix = desync.arg.prefix
	if prefix == "crlf" then
		payload = "\r\n"..payload
	elseif prefix == "space" then
		payload = " "..payload
	elseif prefix == "tab" then
		payload = "\t"..payload
	elseif prefix and #prefix > 0 then
		payload = prefix..payload
	end

	local hostcase = desync.arg.hostcase or "HoSt"
	payload = string.gsub(payload, "Host:", hostcase..":", 1)
	desync.dis.payload = payload
	z2k_log("http_soft modified request")
	return VERDICT_MODIFY
end

-- Strip selected ALPN protocols from TLS ClientHello. This can force HTTP/1.1
-- fallback on hosts where HTTP/2 streams are killed after a clean handshake.
function z2k_alpn_strip(ctx, desync)
	if not desync.dis.tcp then
		if not desync.dis.icmp then instance_cutoff_shim(ctx, desync) end
		return
	end
	direction_cutoff_opposite(ctx, desync)
	if not direction_check(desync) or not payload_check(desync) then return end
	if desync.l7payload ~= "tls_client_hello" then return end

	local data = desync.reasm_data or desync.dis.payload
	if not data or #data == 0 then return end

	local tdis = tls_dissect(data)
	if not tdis or not tdis.handshake or not tdis.handshake[TLS_HANDSHAKE_TYPE_CLIENT] then
		z2k_log("alpn_strip tls_dissect failed")
		return
	end
	local hs = tdis.handshake[TLS_HANDSHAKE_TYPE_CLIENT]
	if not hs.dis or not hs.dis.ext then
		z2k_log("alpn_strip no extensions")
		return
	end

	local idx_alpn = array_field_search(hs.dis.ext, "type", TLS_EXT_ALPN)
	if not idx_alpn then
		z2k_log("alpn_strip no ALPN")
		return
	end
	local alpn = hs.dis.ext[idx_alpn].dis and hs.dis.ext[idx_alpn].dis.list
	if type(alpn) ~= "table" then
		z2k_log("alpn_strip malformed ALPN")
		return
	end

	local strip = {}
	for proto in string.gmatch(desync.arg.strip or "h2,h2c", "[^,]+") do
		strip[proto] = true
	end

	local kept = {}
	local removed = 0
	for _, proto in ipairs(alpn) do
		if strip[proto] then
			removed = removed + 1
		else
			kept[#kept + 1] = proto
		end
	end
	if removed == 0 then return end

	local keep_min = tonumber(desync.arg.keep_min) or 1
	while #kept < keep_min do
		kept[#kept + 1] = "http/1.1"
	end
	hs.dis.ext[idx_alpn].dis.list = kept

	local rtls = tls_reconstruct(tdis)
	if not rtls then
		z2k_log("alpn_strip tls_reconstruct failed")
		return
	end
	desync.dis.payload = rtls
	if desync.reasm_data then
		desync.reasm_data = rtls
	end
	z2k_log("alpn_strip removed "..removed.." protocols")
end

-- Low-TTL QUIC/UDP fake ladder. It sends a few decoys and lets the real
-- packet continue, so a bad experiment should fail softly.
function z2k_quic_fake_ladder(ctx, desync)
	if not desync.dis.udp then
		if not desync.dis.icmp then instance_cutoff_shim(ctx, desync) end
		return
	end
	z2k_resolve_ranges(desync)
	direction_cutoff_opposite(ctx, desync)
	if not direction_check(desync) or not payload_check(desync, "quic_initial") then return end
	if not replay_first(desync) then return end

	local blob_name = desync.arg.blob
	if not blob_name then
		error("z2k_quic_fake_ladder: 'blob' arg required")
	end
	if desync.arg.optional and not blob_exist(desync, blob_name) then
		z2k_log("quic blob missing: "..blob_name)
		return
	end

	local repeats = z2k_clamp(desync.arg.repeats, 1, 12, 4)
	local ttl_start = z2k_clamp(desync.arg.ttl_start, 1, 64, 2)
	local ttl_step = z2k_clamp(desync.arg.ttl_step, 0, 16, 1)
	local fake_payload = blob(desync, blob_name)
	local opts = {
		rawsend = rawsend_opts(desync),
		reconstruct = reconstruct_opts(desync),
		ipfrag = {},
		ipid = desync.arg,
		fooling = desync.arg,
	}
	opts.rawsend.repeats = 1

	for i = 1, repeats do
		local fake = deepcopy(desync.dis)
		fake.payload = fake_payload
		local ttl = ttl_start + (i - 1) * ttl_step
		if fake.ip then fake.ip.ip_ttl = ttl end
		if fake.ip6 then fake.ip6.ip6_hlim = ttl end
		if b_debug then z2k_log("quic_fake_ladder fake #"..i.." ttl="..ttl) end
		if not rawsend_dissect_ipfrag(fake, opts) then
			return VERDICT_PASS
		end
	end
end
