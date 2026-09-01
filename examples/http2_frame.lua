local bpc = require("bpc")

-- HTTP/2 Frame Format (RFC 7540)
-- A perfect example of a binary protocol where one field explicitly defines
-- the byte length of the payload that follows it.
return bpc.layout("http2_frame")
    -- The length of the payload (in bytes)
    :field("length", 24)
    
    -- The frame type (e.g., HEADERS, DATA, SETTINGS)
    :field("type", 8)
    
    -- Frame-specific boolean flags
    :field("flags", 8)
    
    -- 1-bit reserved field followed by a 31-bit stream identifier
    :field("reserved", 1)
    :field("stream_id", 31)
    
    -- The payload whose size is dynamically determined by the `length` field!
    :field_var("payload", "length", "bytes")
    
    :build()
