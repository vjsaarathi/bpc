local bpc = require("bpc")

-- IPv6 Packet Header (RFC 8200)
-- Showcases large fixed-width fields (128-bit addresses) and dynamic payload parsing.
return bpc.layout("ipv6_packet")
    :field("version", 4)
    :field("traffic_class", 8)
    :field("flow_label", 20)
    
    -- Size of the rest of the packet in bytes
    :field("payload_len", 16)
    
    :field("next_header", 8)
    :field("hop_limit", 8)
    
    -- IPv6 Addresses (128 bits each)
    :field("src_addr", 128)
    :field("dst_addr", 128)
    
    -- The packet payload, dynamically sized by `payload_len`
    :field_var("payload", "payload_len", "bytes")
    
    :build()
