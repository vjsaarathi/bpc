local bpc = require("bpc")

-- TCP Header (RFC 793)
-- A great example showcasing heavily packed bit-level fields (like TCP flags).
return bpc.layout("tcp_header")
    :field("src_port", 16)
    :field("dst_port", 16)
    :field("seq_num", 32)
    :field("ack_num", 32)
    
    -- The size of the TCP header in 32-bit words
    :field("data_offset", 4)
    :field("reserved", 3)
    
    -- TCP Flags (1 bit each)
    :field("ns", 1)
    :field("cwr", 1)
    :field("ece", 1)
    :field("urg", 1)
    :field("ack", 1)
    :field("psh", 1)
    :field("rst", 1)
    :field("syn", 1)
    :field("fin", 1)
    
    :field("window_size", 16)
    :field("checksum", 16)
    :field("urg_ptr", 16)
    
    -- Note: TCP Options and Payload require math ((data_offset - 5) * 32 bits) 
    -- which would be a great candidate for future BPC Lua math extensions!
    :build()
