grammar PacketGrammar
import "../common/common";

entry Packet:
  NEWLINE*
  "packet"
  (
    TitleAndAccessibilities
    | blocks+=PacketBlock
    | NEWLINE
  )*
;

PacketBlock:
  (
    start=INT('-' end=INT)?
    | '+' bits=INT
  )
  ':' label=STRING
  EOL
;
