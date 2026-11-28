#!/bin/sh
# Son des tornades de l'ocarina (0.9.1) : flute_1 adoucie (aigus coupes, fondus).
ffmpeg -y -i assets/audio/sfx/flute_1.ogg -af "lowpass=f=2600,afade=t=in:d=0.08,afade=t=out:st=1.7:d=0.8" -c:a libvorbis -q:a 5 assets/audio/sfx/flute_soft.ogg
