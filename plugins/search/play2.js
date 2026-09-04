import { randomUUID } from "crypto";
import { f } from "../../src/lib/ourin-http.js";
import te from "../../src/lib/ourin-error.js";

const pluginConfig = {
  name: "play2",
  alias: ["spotifywebui", "playweb"],
  category: "search",
  description: "Kirim kartu Spotify WebUI interaktif (preview 30 detik) di chat",
  usage: ".play2 <judul lagu>",
  example: ".play2 about you the 1975",
  cooldown: 15,
  energi: 1,
  isEnabled: true,
};

// Port dari AzusaMD spotify.service.js — Deezer prioritas, iTunes fallback
async function searchTrack(query) {
  if (!query) return null;
  const cleanQuery = query
    .replace(
      /https?:\/\/open\.spotify\.com\/(intl-[a-z]+\/)?track\/[a-zA-Z0-9]+(\?.*)?/i,
      "",
    )
    .trim();

  const dz = await f(
    `https://api.deezer.com/search?q=${encodeURIComponent(cleanQuery)}&limit=1`,
  );
  const dzItem = dz?.data?.[0];
  if (dzItem?.preview) {
    return {
      title: dzItem.title || "Unknown Title",
      artist: dzItem.artist?.name || "Unknown Artist",
      album: dzItem.album?.title || "Single",
      coverUrl:
        dzItem.album?.cover_big ||
        dzItem.album?.cover_medium ||
        dzItem.album?.cover_xl ||
        "",
      audioUrl: dzItem.preview,
      trackUrl: dzItem.link || `https://www.deezer.com/track/${dzItem.id}`,
      durationSec: dzItem.duration || 30,
      source: "deezer",
    };
  }

  const it = await f(
    `https://itunes.apple.com/search?term=${encodeURIComponent(cleanQuery)}&media=music&limit=1`,
  );
  const itItem = it?.results?.[0];
  if (itItem?.previewUrl) {
    return {
      title: itItem.trackName || "Unknown Title",
      artist: itItem.artistName || "Unknown Artist",
      album: itItem.collectionName || "Single",
      coverUrl: (itItem.artworkUrl100 || "").replace("100x100bb", "500x500bb"),
      audioUrl: itItem.previewUrl,
      trackUrl: itItem.trackViewUrl || "https://music.apple.com",
      durationSec: Math.max(30, Math.floor((itItem.trackTimeMillis || 30000) / 1000)),
      source: "itunes",
    };
  }
  return null;
}

async function downloadBuffers(track) {
  const [coverBuf, audioBuf] = await Promise.all([
    track.coverUrl ? f(track.coverUrl, "buffer") : null,
    track.audioUrl ? f(track.audioUrl, "buffer") : null,
  ]);
  return { coverBuf, audioBuf };
}

// Port AzusaMD webui.service.js — diselaraskan dengan pola AIRich production (ourin-builder.js)
// yang terbukti terkirim: deviceListMetadata kosong, botMetadata.messageDisclaimerText,
// botJid '0@bot', forwardOrigin 4 (numerik)
async function sendInlineWebUI(sock, jid, htmlPayload, submessageText) {
  const uuid = randomUUID();
  const unifiedResponse = {
    response_id: uuid,
    sections: [
      {
        view_model: {
          primitive: {
            __typename: "GenAIaeacdsnwHtmlPrimitive",
            payload: htmlPayload,
            trusted_sources: [],
          },
          __typename: "GenAISingleLayoutViewModel",
        },
      },
    ],
  };
  const base64Data = Buffer.from(
    JSON.stringify(unifiedResponse),
    "utf-8",
  ).toString("base64");
  const msg = {
    messageContextInfo: {
      deviceListMetadata: {},
      deviceListMetadataVersion: 2,
      botMetadata: {
        messageDisclaimerText: submessageText,
        richResponseSourcesMetadata: { sources: [] },
      },
    },
    botForwardedMessage: {
      message: {
        richResponseMessage: {
          messageType: 1,
          submessages: [
            {
              messageType: 2,
              messageText: submessageText,
            },
          ],
          unifiedResponse: {
            data: base64Data,
          },
          contextInfo: {
            forwardingScore: 1,
            isForwarded: true,
            forwardedAiBotMessageInfo: { botJid: "0@bot" },
            forwardOrigin: 4,
          },
        },
      },
    },
  };
  await sock.relayMessage(jid, msg, {});
  return msg;
}

// Port dari AzusaMD getSpotifyInlineHtml — kartu player HTML5 interaktif
function getPlayerHtml({ title, artist, album, coverUrl, audioUrl, durationSec }) {
  const safeTitle = (title || "Track").replace(/["'<>]/g, "");
  const safeArtist = (artist || "Artist").replace(/["'<>]/g, "");
  const safeAlbum = (album || "Single").replace(/["'<>]/g, "");
  const trackDur = durationSec > 0 ? durationSec : 30;
  const initMin = Math.floor(trackDur / 60);
  const initSec = Math.floor(trackDur % 60);
  const initDurStr = `${initMin}:${initSec < 10 ? "0" : ""}${initSec}`;

  return `<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0, user-scalable=no">
<style>
:root {
  --wa-bg: #111b21;
  --card: #121212;
  --line: rgba(255, 255, 255, 0.08);
  --ink: #e9edef;
  --muted: #a7a7a7;
  --accent: #1db954;
  --accent-hover: #1ed760;
  --sys: -apple-system, BlinkMacSystemFont, 'Circular Std', 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
}
* { margin: 0; padding: 0; box-sizing: border-box; -webkit-tap-highlight-color: transparent; }
html, body {
  background: transparent;
  color: var(--ink);
  font-family: var(--sys);
  min-height: 100vh;
  -webkit-font-smoothing: antialiased;
}
.wrap {
  width: 100%;
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 4px;
  touch-action: pan-y !important;
}
.spotify-card {
  width: 100%;
  max-width: 325px;
  background: #121212;
  border-radius: 24px;
  padding: 16px;
  box-shadow: 0 16px 36px rgba(0, 0, 0, 0.7);
  display: flex;
  flex-direction: column;
  gap: 14px;
  border: 1px solid rgba(255, 255, 255, 0.06);
  user-select: none;
  -webkit-user-select: none;
}
.art-box {
  width: 100%;
  aspect-ratio: 1;
  border-radius: 18px;
  overflow: hidden;
  background: #1e1e1e;
  box-shadow: 0 10px 28px rgba(0, 0, 0, 0.6);
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
}
.art-box img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.meta-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.track-info {
  min-width: 0;
  flex: 1;
}
.track-title {
  font-size: 20px;
  font-weight: 700;
  color: #ffffff;
  line-height: 1.2;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  letter-spacing: -0.4px;
}
.track-artist {
  font-size: 13px;
  font-weight: 500;
  color: var(--muted);
  margin-top: 3px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.quick-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}
.action-pill {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  background: #242424;
  border: none;
  color: #ffffff;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: background .15s, transform .1s, color .15s;
}
.action-pill:active {
  transform: scale(0.92);
  background: #333;
}
.action-pill svg {
  width: 16px;
  height: 16px;
  fill: currentColor;
}
.seekbar-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
  touch-action: none;
}
.bar-track-wrap {
  position: relative;
  width: 100%;
  height: 28px;
  display: flex;
  align-items: center;
  cursor: pointer;
  touch-action: none;
}
.bar-track-bg {
  position: absolute;
  left: 0;
  right: 0;
  height: 6px;
  border-radius: 3px;
  background: #383838;
  pointer-events: none;
}
.bar-track-fill {
  position: absolute;
  left: 0;
  height: 6px;
  border-radius: 3px 0 0 3px;
  background: #1db954;
  width: 0%;
  pointer-events: none;
  transition: width 0.05s linear;
}
.bar-handle {
  position: absolute;
  top: 50%;
  transform: translate(-50%, -50%);
  left: 0%;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: #ffffff;
  box-shadow: 0 0 10px rgba(29, 185, 84, 0.9);
  z-index: 2;
  pointer-events: none;
  transition: transform 0.1s, left 0.05s linear;
}
.bar-track-wrap.dragging .bar-handle {
  transform: translate(-50%, -50%) scale(1.4);
  background: #1db954;
}
.time-labels {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  font-weight: 600;
  color: var(--muted);
  font-variant-numeric: tabular-nums;
  padding: 0 2px;
}
.controls-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 0 2px;
}
.sq-ctrl {
  width: 42px;
  height: 42px;
  border-radius: 12px;
  background: #222222;
  border: none;
  color: #b3b3b3;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all .15s;
}
.sq-ctrl:active {
  transform: scale(0.92);
  color: #fff;
  background: #333;
}
.sq-ctrl.active {
  color: #1db954;
  background: #1c2d22;
}
.sq-ctrl svg {
  width: 18px;
  height: 18px;
  fill: currentColor;
}
.hero-play-btn {
  width: 60px;
  height: 60px;
  border-radius: 50%;
  background: #ffffff;
  color: #000000;
  border: none;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.5);
  transition: transform .15s, background .15s;
}
.hero-play-btn:active {
  transform: scale(0.92);
  background: #e0e0e0;
}
.hero-play-btn svg {
  width: 26px;
  height: 26px;
  fill: #000000;
}
</style>
</head>
<body>
<div class="wrap">
  <div class="spotify-card">
    <div class="art-box">
      <img src="${coverUrl}" onerror="this.style.display='none'; document.getElementById('fb-disc').style.display='flex';">
      <div id="fb-disc" style="display:${coverUrl ? "none" : "flex"};align-items:center;justify-content:center;width:100%;height:100%;background:linear-gradient(135deg,#1f1f1f,#121212);">
        <svg viewBox="0 0 24 24" style="width:64px;height:64px;fill:#404040;"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 14.5c-2.49 0-4.5-2.01-4.5-4.5S9.51 7.5 12 7.5s4.5 2.01 4.5 4.5-2.01 4.5-4.5 4.5zm0-5.5c-.55 0-1 .45-1 1s.45 1 1 1 1-.45 1-1-.45-1-1-1z"/></svg>
      </div>
    </div>

    <div class="meta-row">
      <div class="track-info">
        <div class="track-title">${safeTitle}</div>
        <div class="track-artist">${safeArtist} • ${safeAlbum}</div>
      </div>
      <div class="quick-actions">
        <button class="action-pill" id="heart-btn" title="Like">
          <svg viewBox="0 0 24 24"><path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z"/></svg>
        </button>
      </div>
    </div>

    <div class="seekbar-section">
      <div class="bar-track-wrap" id="seek-wrap">
        <div class="bar-track-bg"></div>
        <div class="bar-track-fill" id="seek-fill"></div>
        <div class="bar-handle" id="seek-handle"></div>
      </div>
      <div class="time-labels">
        <span id="cur-time">0:00</span>
        <span id="dur-time">${initDurStr}</span>
      </div>
    </div>

    <div class="controls-row">
      <button class="sq-ctrl" id="shuffle-btn" title="Shuffle">
        <svg viewBox="0 0 24 24"><path d="M10.59 9.17L5.41 4 4 5.41l5.17 5.17 1.42-1.41zM14.5 4l2.04 2.04L4 18.59 5.41 20 17.96 7.46 20 9.5V4h-5.5zm.33 9.41l-1.41 1.41 3.13 3.13L14.5 20H20v-5.5l-2.04 2.04-3.13-3.13z"/></svg>
      </button>
      <button class="sq-ctrl" id="prev-btn" title="Mundur 10s">
        <svg viewBox="0 0 24 24"><path d="M11 18V6l-8.5 6 8.5 6zm.5-6l8.5 6V6l-8.5 6z"/></svg>
      </button>
      <button class="hero-play-btn" id="play-btn" title="Play">
        <svg viewBox="0 0 24 24" id="play-svg"><polygon points="6 4 20 12 6 20 6 4"/></svg>
      </button>
      <button class="sq-ctrl" id="next-btn" title="Maju 10s">
        <svg viewBox="0 0 24 24"><path d="M4 18l8.5-6L4 6v12zm9-12v12l8.5-6L13 6z"/></svg>
      </button>
      <button class="sq-ctrl active" id="repeat-btn" title="Loop">
        <svg viewBox="0 0 24 24"><path d="M7 7h10v3l4-4-4-4v3H5v6h2V7zm10 10H7v-3l-4 4 4 4v-3h12v-6h-2v4z"/></svg>
      </button>
    </div>
  </div>
</div>

<script>
(function(){
  var btn = document.getElementById('play-btn'),
      fill = document.getElementById('seek-fill'),
      handle = document.getElementById('seek-handle'),
      seekWrap = document.getElementById('seek-wrap'),
      curT = document.getElementById('cur-time'),
      durT = document.getElementById('dur-time'),
      heart = document.getElementById('heart-btn'),
      prevBtn = document.getElementById('prev-btn'),
      nextBtn = document.getElementById('next-btn'),
      shuffleBtn = document.getElementById('shuffle-btn'),
      repeatBtn = document.getElementById('repeat-btn');

  var playing = false,
      liked = false,
      loop = true,
      shuffle = false,
      isDragging = false,
      curSec = 0,
      totalSec = ${trackDur};

  var rawAudio = "${audioUrl}";
  var nativeAudio = null;
  var actx = null, synthTimer = null;

  if (rawAudio) {
    try {
      nativeAudio = new Audio();
      nativeAudio.src = rawAudio;
      nativeAudio.preload = 'metadata';
      nativeAudio.onloadedmetadata = function(){
        if (nativeAudio.duration && isFinite(nativeAudio.duration) && nativeAudio.duration > 0) {
          totalSec = Math.floor(nativeAudio.duration);
          var m = Math.floor(totalSec / 60), s = Math.floor(totalSec % 60);
          durT.innerText = m + ':' + (s < 10 ? '0' : '') + s;
        }
      };
      nativeAudio.ontimeupdate = function(){
        if (!isDragging && playing) {
          curSec = Math.floor(nativeAudio.currentTime);
          updateProgress(curSec);
        }
      };
      nativeAudio.onended = function(){
        if (loop) {
          seekTo(0);
          nativeAudio.play();
        } else {
          setPlayState(false);
          seekTo(0);
        }
      };
    } catch(e){}
  }

  var NOTES = [261.63, 293.66, 329.63, 349.23, 392.00, 440.00, 523.25];
  var CHORDS = [
    [261.63, 329.63, 392.00],
    [220.00, 261.63, 329.63],
    [174.61, 220.00, 261.63],
    [196.00, 246.94, 293.66]
  ];

  function playSynthStep(stepIndex){
    if (!actx || actx.state === 'suspended') return;
    try {
      var now = actx.currentTime;
      var chordIdx = Math.floor((stepIndex / 4) % CHORDS.length);
      var chord = CHORDS[chordIdx];

      if (stepIndex % 4 === 0) {
        chord.forEach(function(freq){
          var osc = actx.createOscillator();
          var gain = actx.createGain();
          osc.type = 'triangle';
          osc.frequency.setValueAtTime(freq, now);
          gain.gain.setValueAtTime(0.04, now);
          gain.gain.exponentialRampToValueAtTime(0.001, now + 1.8);
          osc.connect(gain);
          gain.connect(actx.destination);
          osc.start(now);
          osc.stop(now + 1.9);
        });
      }

      var melFreq = NOTES[(stepIndex * 3 + chordIdx) % NOTES.length];
      var leadOsc = actx.createOscillator();
      var leadGain = actx.createGain();
      leadOsc.type = 'sine';
      leadOsc.frequency.setValueAtTime(melFreq, now);
      leadGain.gain.setValueAtTime(0.08, now);
      leadGain.gain.exponentialRampToValueAtTime(0.001, now + 0.4);
      leadOsc.connect(leadGain);
      leadGain.connect(actx.destination);
      leadOsc.start(now);
      leadOsc.stop(now + 0.45);
    } catch(e){}
  }

  function startSynthEngine(){
    if (!actx) actx = new (window.AudioContext || window.webkitAudioContext)();
    if (actx.state === 'suspended') actx.resume();

    if (synthTimer) clearInterval(synthTimer);
    var step = Math.floor(curSec * 2);

    synthTimer = setInterval(function(){
      if (!playing) return;
      playSynthStep(step);
      step++;
      if (step % 2 === 0) {
        curSec++;
        if (curSec >= totalSec) {
          if (loop) {
            seekTo(0);
            step = 0;
          } else {
            setPlayState(false);
            seekTo(0);
            return;
          }
        }
        if (!isDragging) updateProgress(curSec);
      }
    }, 500);
  }

  function stopSynthEngine(){
    if (synthTimer) { clearInterval(synthTimer); synthTimer = null; }
  }

  function updateProgress(sec){
    var pct = Math.min(100, Math.max(0, (sec / totalSec) * 100));
    fill.style.width = pct + '%';
    handle.style.left = pct + '%';
    var m = Math.floor(sec / 60), s = Math.floor(sec % 60);
    curT.innerText = m + ':' + (s < 10 ? '0' : '') + s;
  }

  function seekTo(sec){
    curSec = Math.min(totalSec, Math.max(0, sec));
    updateProgress(curSec);
    if (nativeAudio) {
      try { nativeAudio.currentTime = curSec; } catch(e){}
    }
  }

  function handleSeekPosition(clientX){
    var rect = seekWrap.getBoundingClientRect();
    var clickX = clientX - rect.left;
    var pct = Math.min(1, Math.max(0, clickX / rect.width));
    seekTo(Math.floor(pct * totalSec));
  }

  seekWrap.addEventListener('pointerdown', function(e){
    isDragging = true;
    seekWrap.classList.add('dragging');
    seekWrap.setPointerCapture(e.pointerId);
    handleSeekPosition(e.clientX);
  });

  seekWrap.addEventListener('pointermove', function(e){
    if (isDragging) handleSeekPosition(e.clientX);
  });

  seekWrap.addEventListener('pointerup', function(e){
    if (isDragging) {
      isDragging = false;
      seekWrap.classList.remove('dragging');
      try { seekWrap.releasePointerCapture(e.pointerId); } catch(e){}
      if (playing && nativeAudio) nativeAudio.play();
    }
  });

  seekWrap.addEventListener('pointercancel', function(){
    isDragging = false;
    seekWrap.classList.remove('dragging');
  });

  function setPlayState(isPlay){
    playing = isPlay;
    if (playing) {
      btn.innerHTML = '<svg viewBox="0 0 24 24" style="width:26px;height:26px;fill:#000;"><rect x="6" y="4" width="4" height="16"/><rect x="14" y="4" width="4" height="16"/></svg>';
      if (nativeAudio && nativeAudio.src) {
        nativeAudio.play().catch(function(){
          startSynthEngine();
        });
      } else {
        startSynthEngine();
      }
    } else {
      btn.innerHTML = '<svg viewBox="0 0 24 24" style="width:26px;height:26px;fill:#000;"><polygon points="6 4 20 12 6 20 6 4"/></svg>';
      if (nativeAudio) nativeAudio.pause();
      stopSynthEngine();
    }
  }

  btn.onclick = function(){ setPlayState(!playing); };

  prevBtn.onclick = function(){ seekTo(curSec > 10 ? curSec - 10 : 0); };
  nextBtn.onclick = function(){ seekTo(curSec + 10); };

  repeatBtn.onclick = function(){
    loop = !loop;
    repeatBtn.classList.toggle('active', loop);
  };

  shuffleBtn.onclick = function(){
    shuffle = !shuffle;
    shuffleBtn.classList.toggle('active', shuffle);
  };

  heart.onclick = function(){
    liked = !liked;
    heart.style.color = liked ? '#1db954' : '#ffffff';
  };

  updateProgress(0);
})();
</script>
</body>
</html>
`;
}

async function handler(m, { sock }) {
  const query = m.text?.trim();
  if (!query)
    return m.reply(`⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n> \`${m.prefix}play2 <judul lagu>\`\n\nKartu Spotify WebUI interaktif dengan preview audio 30 detik.\n\nContoh: \`${m.prefix}play2 the shade\``);

  await m.react("🕕");

  try {
    const track = await searchTrack(query);
    if (!track) {
      await m.react("❌");
      return m.reply(`❌ Lagu "${query}" tidak ditemukan. Coba judul atau nama artis yang lebih spesifik.`);
    }

    const { coverBuf, audioBuf } = await downloadBuffers(track);

    // ponytail: embed base64 hanya jika buffer kecil — node WA drop payload >±1MB
    // audio embed >600KB → pakai URL preview langsung agar pesan pasti sampai
    const base64Cover = coverBuf
      ? `data:image/jpeg;base64,${coverBuf.toString("base64")}`
      : track.coverUrl || "";
    const base64Audio =
      audioBuf && audioBuf.length <= 600_000
        ? `data:audio/mp3;base64,${audioBuf.toString("base64")}`
        : track.audioUrl || "";

    const playerHtml = getPlayerHtml({
      title: track.title,
      artist: track.artist,
      album: track.album,
      coverUrl: base64Cover,
      audioUrl: base64Audio,
      durationSec: track.durationSec || 30,
    });

    await sendInlineWebUI(sock, m.chat, playerHtml, `${track.title} - ${track.artist}`);
    await m.react("✅");
  } catch (e) {
    console.error("[Play2 Error]", e);
    await m.react("☢");
    m.reply(te(m.prefix, m.command, m.pushName));
  }
}

export { pluginConfig as config, handler };
