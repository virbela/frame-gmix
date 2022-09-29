#!/bin/bash

gst-launch-1.0 -vv udpsrc port=1925 caps="application/x-rtp,media=(string)audio,clock-rate=(int)48000,encoding-name=(string)OPUS"  ! rtpbin ! rtpopusdepay ! opusparse ! audiomixer ! oggmux ! filesink location=james.ogg



gst-launch-1.0 -vv udpsrc port=1925 caps="application/x-rtp,media=(string)audio,clock-rate=(int)48000,encoding-name=(string)OPUS"  ! queue ! rtpbin ! rtpopusdepay ! opusparse ! opusdec ! audiomixer ! opusenc ! opusparse ! oggmux ! filesink location=james.ogg

