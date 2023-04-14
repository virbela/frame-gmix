# frame-gmix
GStreamer-backed audio mixing application designed to handle multiple incoming RTP audio streams and mix to a RTP output.

# gst commandline
###pub
gst-launch-1.0 filesrc location=test.opus ! decodebin ! audioconvert ! opusenc ! rtpopuspay ! udpsink host=127.0.0.1 port=5085

###listen
gst-launch-1.0 udpsrc port=6000 caps="application/x-rtp, media=(string)audio, clock-rate=(int)48000, encoding-name=(string)OPUS" ! rtpopusdepay ! opusdec ! audioconvert ! autoaudiosink