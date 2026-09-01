local bpc = require("bpc")

return bpc.layout("network_packet")
    :field("version", 4)
    :field("flags", 4)
    :field("length", 8)
    :field_var("payload", "length", "bytes")
    :build()
