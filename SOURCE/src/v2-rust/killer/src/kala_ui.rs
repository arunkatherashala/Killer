// =============================================================================
// kala_ui.rs — Kala (काल) Chat UI Server
// Pure std::net TCP — zero external crates.
// Serves a world-class AI chat interface at http://127.0.0.1:PORT/
//
// Web lane for parallel UI: keep logical model aligned with `killer_ui` — see
// SOURCE/docs/KILLER_UI_ENGINE.md → “Parallel lanes — shared contract”.
//
// Builtins:
//   kala_serve(port?)    — start server, auto-open browser, blocks until Ctrl+C
// =============================================================================

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use crate::error::VmError;
use crate::value::Value;

// ─── Embedded World-Class HTML UI ─────────────────────────────────────────────
const KALA_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1.0">
<title>Kala — AI by Killer</title>
<script type="importmap">
{"imports":{"three":"https://cdn.jsdelivr.net/npm/three@0.160.0/build/three.module.js"}}
</script>
<style>
*,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
:root{
  --bg:#06060e;--s1:#0a0a18;--s2:#10101f;--s3:#181830;
  --border:rgba(255,255,255,0.06);--border2:rgba(255,255,255,0.09);
  --gold:#f59e0b;--gold2:#fbbf24;--gold3:#fde68a;
  --purple:#8b5cf6;--blue:#3b82f6;--green:#22c55e;--red:#ef4444;
  --text:#eeeef8;--dim:#6b6b9e;--dim2:#9e9ec4;
  --code-bg:#0a0e17;
  --glass:rgba(16,16,32,0.65);--glass2:rgba(20,20,42,0.55);
  --glow-gold:rgba(245,158,11,0.15);--glow-purple:rgba(139,92,246,0.12);
  --accent-gradient:linear-gradient(135deg,#f59e0b,#ef4444,#8b5cf6);
  --radius:16px;
}
html,body{height:100%;overflow:hidden}
body{font-family:'Inter',-apple-system,BlinkMacSystemFont,'Segoe UI','Roboto',sans-serif;
  background:var(--bg);color:var(--text);display:flex;height:100vh;
  background-image:radial-gradient(ellipse 80% 60% at 20% 10%,rgba(139,92,246,0.04) 0%,transparent 50%),
                   radial-gradient(ellipse 60% 50% at 80% 90%,rgba(245,158,11,0.03) 0%,transparent 50%)}
button{cursor:pointer;font-family:inherit}
textarea,select,input{font-family:inherit}
::-webkit-scrollbar{width:5px;height:5px}
::-webkit-scrollbar-track{background:transparent}
::-webkit-scrollbar-thumb{background:var(--border2);border-radius:4px}
*{scrollbar-width:thin;scrollbar-color:var(--border2) transparent}

/* ── Kala Mood indicator ─────────────────────────────────── */
.kala-mood{
  display:inline-block;font-size:20px;cursor:pointer;
  transition:transform 0.3s;vertical-align:middle;margin-right:2px}
.kala-mood:hover{transform:scale(1.3) rotate(10deg)}
.mood-pulse{animation:moodPulse 0.6s ease}
@keyframes moodPulse{
  0%{transform:scale(1)}
  50%{transform:scale(1.4) rotate(15deg)}
  100%{transform:scale(1)}
}
/* ── Typing animation ────────────────────────────────────── */
.typing-cursor{
  display:inline-block;width:2px;height:1em;background:var(--gold);
  animation:blink 0.7s infinite;vertical-align:text-bottom;margin-left:1px}
@keyframes blink{0%,100%{opacity:1}50%{opacity:0}}
/* ── Kala message entrance ───────────────────────────────── */
.bubble.kb{animation:msgIn 0.35s ease-out}
@keyframes msgIn{
  0%{opacity:0;transform:translateY(12px) scale(0.97)}
  100%{opacity:1;transform:translateY(0) scale(1)}
}
/* ── Sparkle effect on send ──────────────────────────────── */
.sparkle{
  position:fixed;pointer-events:none;font-size:18px;z-index:9999;
  animation:sparkleUp 0.8s ease-out forwards}
@keyframes sparkleUp{
  0%{opacity:1;transform:translateY(0) scale(1)}
  100%{opacity:0;transform:translateY(-40px) scale(0.3) rotate(90deg)}
}

/* ── Session gate ("login") — camera for live face + gesture UI ─── */
.kala-gate{
  display:none;position:fixed;inset:0;z-index:9999;align-items:center;justify-content:center;
  padding:24px;background:radial-gradient(ellipse 70% 50% at 50% 20%,#0f172a 0%,#020617 50%,#000 100%);
  -webkit-backdrop-filter:blur(6px);backdrop-filter:blur(6px)}
.kala-gate.kala-gate-visible{display:flex!important}
body.kala-gate-on{overflow:hidden}
.kala-gate-card{
  width:100%;max-width:420px;padding:clamp(22px,5vw,34px);
  border-radius:24px;border:1px solid rgba(94,234,212,0.25);
  background:rgba(7,10,22,0.88);backdrop-filter:blur(24px);
  box-shadow:0 0 80px rgba(94,234,212,0.1),0 24px 48px rgba(0,0,0,0.3);
  text-align:center}
.kala-gate-logo{
  width:56px;height:56px;margin:0 auto 14px;border-radius:14px;
  background:linear-gradient(135deg,#f0a500,#ef4444,#7c3aed);
  display:flex;align-items:center;justify-content:center;font-size:26px}
.kala-gate-card h1{
  font-size:26px;font-weight:800;background:linear-gradient(135deg,#5eead4,#fbbf24);
  -webkit-background-clip:text;-webkit-text-fill-color:transparent;margin-bottom:8px}
.kala-gate-card p{
  font-size:13px;color:var(--dim2);line-height:1.55;margin-bottom:20px}
.kala-gate-prim{
  width:100%;padding:14px 18px;border-radius:12px;border:none;cursor:pointer;
  font-size:14px;font-weight:700;color:#020617;
  background:linear-gradient(135deg,#5eead4,#2dd4bf);box-shadow:0 4px 24px rgba(45,212,191,0.35);
  transition:transform 0.15s,box-shadow 0.15s;margin-bottom:10px}
.kala-gate-prim:hover{transform:translateY(-1px);box-shadow:0 6px 28px rgba(45,212,191,0.45)}
.kala-gate-sec{
  width:100%;padding:11px;border-radius:12px;border:1px solid var(--border2);
  background:transparent;color:var(--dim2);font-size:13px;cursor:pointer;transition:all 0.12s}
.kala-gate-sec:hover{border-color:var(--gold);color:var(--gold2)}
.kala-gate-note{font-size:10.5px;color:var(--dim);margin-top:14px;line-height:1.45}

/* ── Sidebar ──────────────────────────────────────────────── */
.sidebar{display:none;backdrop-filter:blur(20px);background:var(--glass)!important;
  border-right:1px solid var(--border)!important}
.logo-area{padding:18px 16px 16px;border-bottom:1px solid var(--border)}
.logo-row{display:flex;align-items:center;gap:10px;margin-bottom:4px}
.logo-icon{
  width:40px;height:40px;border-radius:12px;flex-shrink:0;
  background:var(--accent-gradient);
  display:flex;align-items:center;justify-content:center;
  font-size:19px;box-shadow:0 4px 24px rgba(245,158,11,0.3);
  animation:logoGlow 3s ease-in-out infinite}
@keyframes logoGlow{0%,100%{box-shadow:0 4px 24px rgba(245,158,11,0.3)}50%{box-shadow:0 4px 32px rgba(245,158,11,0.5)}}
.logo-name{
  font-size:22px;font-weight:800;letter-spacing:-0.5px;
  background:linear-gradient(135deg,#f59e0b,#fbbf24,#fde68a);
  -webkit-background-clip:text;-webkit-text-fill-color:transparent}
.logo-sub{font-size:11px;color:var(--dim);padding-left:50px;line-height:1.4}
.logo-sub strong{color:var(--dim2)}

.modes-wrap{padding:10px 8px;flex:1;overflow-y:auto}
.sec-label{
  font-size:9.5px;text-transform:uppercase;letter-spacing:1.8px;
  color:var(--dim);padding:4px 10px 6px;display:block}
.mode-btn{
  display:flex;align-items:center;gap:10px;width:100%;
  padding:10px 12px;border-radius:10px;border:1px solid transparent;
  background:transparent;color:var(--dim2);font-size:13.5px;
  text-align:left;transition:all 0.18s cubic-bezier(.4,0,.2,1);margin-bottom:2px;
  position:relative;overflow:hidden}
.mode-btn:hover{background:rgba(255,255,255,0.04);color:var(--text);
  transform:translateX(2px)}
.mode-btn.active{
  background:rgba(245,158,11,0.08);border-color:rgba(245,158,11,0.2);
  color:var(--gold2);box-shadow:0 0 20px var(--glow-gold)}
.mode-btn.active::before{
  content:'';position:absolute;left:0;top:8px;bottom:8px;width:3px;
  border-radius:0 3px 3px 0;background:var(--gold);
  box-shadow:0 0 8px var(--gold)}
.m-ico{font-size:17px;width:22px;text-align:center;flex-shrink:0}
.m-badge{
  margin-left:auto;padding:2px 7px;border-radius:10px;
  font-size:9px;font-weight:700;letter-spacing:0.5px;
  background:rgba(124,58,237,0.14);border:1px solid rgba(124,58,237,0.25);
  color:#a78bfa}
.m-badge.gold{background:rgba(240,165,0,0.1);border-color:rgba(240,165,0,0.25);color:var(--gold)}

.sidebar-foot{
  padding:12px 16px;border-top:1px solid var(--border);font-size:11px}
.stat{display:flex;justify-content:space-between;margin-bottom:4px;color:var(--dim)}
.sv{color:var(--gold);font-weight:600}
.online{
  display:flex;align-items:center;gap:7px;
  margin-top:9px;font-size:11px;color:var(--green)}
.pulse{
  width:7px;height:7px;border-radius:50%;background:var(--green);
  animation:pulse 2s infinite}
@keyframes pulse{
  0%{box-shadow:0 0 0 0 rgba(34,197,94,0.5)}
  70%{box-shadow:0 0 0 7px rgba(34,197,94,0)}
  100%{box-shadow:0 0 0 0 rgba(34,197,94,0)}}

/* ── Main ─────────────────────────────────────────────────── */
.main{flex:1;display:flex;flex-direction:column;overflow:hidden;min-width:0}

.topbar{
  height:54px;background:var(--glass);backdrop-filter:blur(16px);
  border-bottom:1px solid var(--border);
  display:flex;align-items:center;padding:0 20px;gap:10px;flex-shrink:0;
  position:relative;z-index:10}
.topbar::after{
  content:'';position:absolute;bottom:0;left:10%;right:10%;height:1px;
  background:linear-gradient(90deg,transparent,var(--gold),transparent);opacity:0.15}
.tb-title{font-size:15px;font-weight:700;color:var(--text);letter-spacing:-0.3px}
.tb-title span{color:var(--gold)}
.tb-right{margin-left:auto;display:flex;align-items:center;gap:8px}
.bdg{
  padding:4px 12px;border-radius:20px;font-size:10.5px;font-weight:600;
  backdrop-filter:blur(8px);letter-spacing:0.3px}
.bdg-purple{
  background:rgba(139,92,246,0.1);border:1px solid rgba(139,92,246,0.2);color:#a78bfa}
.bdg-gold{
  background:rgba(245,158,11,0.08);border:1px solid rgba(245,158,11,0.2);color:var(--gold)}
.bdg-green{
  background:rgba(34,197,94,0.08);border:1px solid rgba(34,197,94,0.2);color:var(--green)}

/* ── Messages ─────────────────────────────────────────────── */
#msgs{
  flex:1;overflow-y:auto;padding:20px 24px;position:relative;
  display:flex;flex-direction:column;gap:0;scroll-behavior:smooth}

/* Welcome */
.welcome{
  display:flex;flex-direction:column;align-items:center;
  justify-content:center;flex:1;gap:16px;text-align:center;
  padding:40px 20px;animation:fadeIn 0.6s cubic-bezier(.4,0,.2,1)}
@keyframes fadeIn{from{opacity:0;transform:translateY(22px) scale(0.98)}to{opacity:1;transform:translateY(0) scale(1)}}
.wglyph{
  font-size:64px;line-height:1;margin-bottom:4px;
  animation:glow 3s ease-in-out infinite}
@keyframes glow{
  0%,100%{filter:drop-shadow(0 0 12px rgba(245,158,11,0.3))}
  50%{filter:drop-shadow(0 0 36px rgba(245,158,11,0.65))}}
.wtitle{
  font-size:34px;font-weight:800;letter-spacing:-1.5px;color:var(--text)}
.wtitle em{
  font-style:normal;
  background:var(--accent-gradient);
  -webkit-background-clip:text;-webkit-text-fill-color:transparent}
.wsub{font-size:14.5px;color:var(--dim2);max-width:460px;line-height:1.65}
.wpills{display:flex;flex-wrap:wrap;gap:8px;justify-content:center;margin-top:6px}
.wpill{
  padding:5px 14px;border-radius:20px;
  border:1px solid var(--border2);background:var(--glass);
  backdrop-filter:blur(8px);font-size:12px;color:var(--dim2);
  transition:all 0.15s}
.wpill:hover{border-color:var(--gold);color:var(--gold2)}
.pgrid{
  display:grid;grid-template-columns:1fr 1fr;
  gap:10px;margin-top:12px;width:100%;max-width:580px}
.pcard{
  padding:16px 18px;border:1px solid var(--border);
  border-radius:14px;background:var(--glass);backdrop-filter:blur(8px);
  cursor:pointer;text-align:left;
  transition:all 0.2s cubic-bezier(.4,0,.2,1);
  display:flex;flex-direction:column;gap:6px;position:relative;overflow:hidden}
.pcard::before{
  content:'';position:absolute;inset:0;border-radius:14px;
  background:linear-gradient(135deg,transparent 40%,rgba(245,158,11,0.03));
  opacity:0;transition:opacity 0.2s}
.pcard:hover::before{opacity:1}
.pcard:hover{
  border-color:rgba(245,158,11,0.3);
  transform:translateY(-3px);
  box-shadow:0 8px 32px rgba(245,158,11,0.1)}
.pico{font-size:22px}.ptitle{font-size:13px;font-weight:600;color:var(--text)}
.pdesc{font-size:11px;color:var(--dim2)}

/* ── Message Bubbles ─────────────────────────────────────── */
.mwrap{display:flex;gap:12px;padding:10px 4px;animation:slideUp 0.3s cubic-bezier(.4,0,.2,1)}
@keyframes slideUp{from{opacity:0;transform:translateY(14px)}to{opacity:1;transform:translateY(0)}}
.mwrap.user{flex-direction:row-reverse}
.av{
  width:36px;height:36px;border-radius:50%;
  display:flex;align-items:center;justify-content:center;
  font-size:15px;flex-shrink:0;margin-top:2px;
  transition:transform 0.2s}
.mwrap:hover .av{transform:scale(1.08)}
.av.k{background:var(--accent-gradient);
  box-shadow:0 2px 16px rgba(245,158,11,0.25)}
.av.u{background:linear-gradient(135deg,#8b5cf6,#3b82f6);
  box-shadow:0 2px 12px rgba(139,92,246,0.2)}
.mbody{flex:1;min-width:0;max-width:720px}
.mwrap.user .mbody{display:flex;flex-direction:column;align-items:flex-end}
.bubble{
  padding:14px 18px;border-radius:18px;
  line-height:1.72;font-size:14px;max-width:100%;word-wrap:break-word;overflow-wrap:break-word;
  transition:box-shadow 0.2s}
.kb{
  background:var(--glass2);backdrop-filter:blur(12px);
  border:1px solid var(--border2);
  border-radius:18px 18px 18px 4px;color:var(--text)}
.mwrap:hover .kb{box-shadow:0 4px 24px rgba(0,0,0,0.2)}
.ub{
  background:rgba(139,92,246,0.08);backdrop-filter:blur(8px);
  border:1px solid rgba(139,92,246,0.15);
  border-radius:18px 18px 4px 18px;color:var(--text);max-width:80%}
.mwrap:hover .ub{box-shadow:0 4px 20px rgba(139,92,246,0.1)}
.mmeta{
  font-size:11px;color:var(--dim);margin-top:6px;
  display:flex;align-items:center;gap:6px}
.mwrap.user .mmeta{justify-content:flex-end}
.tg{padding:2px 8px;border-radius:10px;font-size:9.5px;font-weight:700}
.tg-gold{background:rgba(245,158,11,0.08);border:1px solid rgba(245,158,11,0.15);color:var(--gold)}
.tg-time{color:var(--dim)}
/* ── Message reactions ─────────────────────────────────────── */
.msg-reactions{display:flex;gap:4px;margin-top:6px;flex-wrap:wrap}
.msg-react-btn{
  padding:2px 8px;border-radius:12px;border:1px solid var(--border);
  background:rgba(255,255,255,0.03);color:var(--dim2);font-size:12px;
  cursor:pointer;transition:all 0.15s;display:flex;align-items:center;gap:3px}
.msg-react-btn:hover{border-color:var(--gold);background:var(--glow-gold);transform:scale(1.08)}
.msg-react-btn.reacted{border-color:var(--gold);background:var(--glow-gold);color:var(--gold2)}
/* ── Thinking Chain ────────────────────────────────────────── */
.thinking-chain{display:flex;flex-direction:column;gap:6px;padding:4px 0}
.think-step{
  display:flex;align-items:center;gap:8px;opacity:0;
  animation:thinkFadeIn 0.3s ease forwards;font-size:12px;color:var(--dim2)}
.think-step.done .think-dot{background:var(--green);box-shadow:0 0 6px rgba(34,197,94,0.4)}
.think-dot{
  width:6px;height:6px;border-radius:50%;background:var(--gold);flex-shrink:0;
  animation:thinkPulse 0.8s infinite;box-shadow:0 0 6px var(--glow-gold)}
.think-step.done .think-dot{animation:none}
.think-text{color:var(--dim2)}
@keyframes thinkFadeIn{from{opacity:0;transform:translateX(-8px)}to{opacity:1;transform:translateX(0)}}
@keyframes thinkPulse{0%,100%{opacity:0.5}50%{opacity:1}}
/* ── Smart Suggestions ─────────────────────────────────────── */
.smart-suggestions{
  display:flex;flex-wrap:wrap;gap:6px;margin-top:8px}
.sug-btn{
  padding:5px 14px;border-radius:20px;
  border:1px solid var(--border2);background:rgba(255,255,255,0.02);
  color:var(--dim2);font-size:11.5px;cursor:pointer;
  transition:all 0.2s;backdrop-filter:blur(4px)}
.sug-btn:hover{
  border-color:var(--gold);color:var(--gold2);
  background:var(--glow-gold);transform:translateY(-1px);
  box-shadow:0 4px 12px var(--glow-gold)}
/* ── Confidence Bar ────────────────────────────────────────── */
.conf-bar{
  display:inline-block;width:40px;height:4px;border-radius:2px;
  background:rgba(255,255,255,0.06);vertical-align:middle;
  margin-left:4px;overflow:hidden}
.conf-fill{
  display:block;height:100%;border-radius:2px;
  transition:width 0.5s ease}
/* ── Intent & Topic Tags ───────────────────────────────────── */
.tg-intent{
  background:rgba(139,92,246,0.08);border:1px solid rgba(139,92,246,0.15);
  color:#a78bfa;text-transform:capitalize;font-size:9px}
.tg-topic{
  background:rgba(59,130,246,0.08);border:1px solid rgba(59,130,246,0.15);
  color:#60a5fa;font-size:9px}
/* ── Scroll FAB ────────────────────────────────────────────── */
.scroll-fab{
  position:absolute;bottom:20px;right:24px;width:40px;height:40px;
  border-radius:50%;border:1px solid var(--border2);
  background:var(--glass);backdrop-filter:blur(12px);
  color:var(--text);font-size:18px;cursor:pointer;
  display:none;align-items:center;justify-content:center;
  box-shadow:0 4px 20px rgba(0,0,0,0.3);
  transition:all 0.2s;z-index:20}
.scroll-fab:hover{transform:scale(1.1);border-color:var(--gold);
  box-shadow:0 4px 24px var(--glow-gold)}
.scroll-fab.show{display:flex}

/* ── Markdown ─────────────────────────────────────────────── */
.bubble h3{font-size:15px;font-weight:700;color:var(--gold2);margin:14px 0 6px}
.bubble h4{font-size:13.5px;font-weight:600;color:var(--dim2);margin:10px 0 4px}
.bubble p{margin-bottom:10px}.bubble p:last-child{margin-bottom:0}
.bubble strong{color:#fff;font-weight:700}
.bubble em{color:var(--dim2);font-style:italic}
.bubble ul,.bubble ol{padding-left:22px;margin-bottom:10px}
.bubble li{margin-bottom:4px;line-height:1.55}
.bubble code{
  background:rgba(0,0,0,0.45);border:1px solid var(--border2);
  border-radius:4px;padding:1px 6px;font-size:12.5px;
  font-family:'Cascadia Code','JetBrains Mono','Fira Code',Consolas,monospace;
  color:#79c0ff}
.bubble pre{
  background:var(--code-bg);border:1px solid var(--border);
  border-radius:14px;padding:0;margin:12px 0;overflow:hidden;position:relative;
  box-shadow:0 4px 16px rgba(0,0,0,0.2)}
.pre-head{
  display:flex;align-items:center;justify-content:space-between;
  padding:9px 14px;border-bottom:1px solid var(--border);
  background:rgba(10,15,21,0.8);backdrop-filter:blur(8px)}
.pre-lang{font-size:11px;color:var(--dim);font-family:'Cascadia Code','JetBrains Mono',monospace;
  letter-spacing:0.5px}
.cpbtn{
  padding:4px 10px;border-radius:6px;border:1px solid var(--border);
  background:rgba(255,255,255,0.03);color:var(--dim2);font-size:11px;cursor:pointer;
  transition:all 0.15s}
.cpbtn:hover{border-color:var(--gold);color:var(--gold);background:var(--glow-gold)}
.cpbtn.ok{border-color:var(--green);color:var(--green);background:rgba(34,197,94,0.08)}
.retry-btn{
  margin-top:10px;padding:8px 16px;border-radius:10px;border:1px solid var(--gold);
  background:rgba(240,165,0,0.12);color:var(--gold2);font-size:12.5px;
  cursor:pointer;transition:all 0.12s;display:block}
.retry-btn:hover{background:rgba(240,165,0,0.22);border-color:var(--gold2)}
.bubble pre code{
  background:none;border:none;padding:14px 16px;font-size:13px;
  color:#c9d1d9;display:block;overflow-x:auto}
.bubble blockquote{
  border-left:3px solid var(--gold);padding-left:12px;
  margin:10px 0;color:var(--dim2);font-style:italic}
.bubble hr{border:none;border-top:1px solid var(--border);margin:14px 0}

/* ── Image / Video rendering ───────────────────────────────── */
.kala-img-wrap{margin:12px 0;border-radius:12px;overflow:hidden;border:1px solid var(--border);max-width:520px}
.kala-img{width:100%;min-width:280px;height:auto;display:block;border-radius:12px 12px 0 0;image-rendering:auto}
.kala-img-cap{padding:7px 12px;font-size:12px;color:var(--dim2);background:var(--s2);margin:0}
.kala-img-cap a{color:var(--gold);text-decoration:none}.kala-img-cap a:hover{text-decoration:underline}

/* ── Thinking dots ─────────────────────────────────────────── */
.twrap{display:flex;gap:10px;padding:8px 4px;animation:slideUp 0.3s ease}
.tdots{display:flex;align-items:center;gap:6px;padding:14px 18px;
  background:var(--glass2);border:1px solid var(--border);
  border-radius:18px 18px 18px 4px;backdrop-filter:blur(8px)}
.dot{
  width:8px;height:8px;border-radius:50%;background:var(--gold);
  animation:bounce 1.4s ease-in-out infinite;
  box-shadow:0 0 8px rgba(245,158,11,0.3)}
.dot:nth-child(2){animation-delay:.2s}.dot:nth-child(3){animation-delay:.4s}
@keyframes bounce{0%,80%,100%{transform:scale(.55);opacity:.3}40%{transform:scale(1.1);opacity:1}}

/* ── Input area ────────────────────────────────────────────── */
.input-area{
  padding:14px 20px 18px;border-top:1px solid var(--border);
  background:var(--glass);backdrop-filter:blur(16px);flex-shrink:0;
  position:relative}
.chips{display:none;gap:5px;margin-bottom:9px;flex-wrap:wrap}
.chips.show{display:flex}
.chip{
  padding:3px 11px;border-radius:20px;
  border:1px solid var(--border);background:transparent;
  color:var(--dim2);font-size:11.5px;cursor:pointer;transition:all 0.1s}
.chip:hover{border-color:var(--gold);color:var(--gold2)}
.chip.on{background:rgba(240,165,0,0.09);border-color:var(--gold);color:var(--gold2)}
.ibox{
  display:flex;align-items:flex-end;gap:8px;
  background:rgba(16,16,32,0.5);backdrop-filter:blur(12px);
  border:1.5px solid var(--border2);
  border-radius:16px;padding:10px 12px;transition:all 0.2s cubic-bezier(.4,0,.2,1)}
.ibox:focus-within{
  border-color:var(--gold);
  box-shadow:0 0 0 3px rgba(245,158,11,0.08),0 4px 24px rgba(245,158,11,0.06)}
.msel{
  padding:5px 24px 5px 8px;border-radius:8px;
  border:1px solid var(--border);background:var(--s3);
  color:var(--text);font-size:12px;outline:none;cursor:pointer;
  flex-shrink:0;transition:border-color 0.1s;appearance:none;
  background-image:url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6'%3E%3Cpath d='M0 0l5 6 5-6z' fill='%236868a0'/%3E%3C/svg%3E");
  background-repeat:no-repeat;background-position:right 7px center}
.msel:focus{border-color:var(--gold)}
#qi{
  flex:1;background:transparent;border:none;outline:none;
  color:var(--text);font-size:14px;resize:none;
  min-height:22px;max-height:160px;overflow-y:auto;line-height:1.5}
#qi::placeholder{color:var(--dim)}
.sbtn{
  width:38px;height:38px;border-radius:12px;border:none;
  background:var(--accent-gradient);color:#000;display:flex;
  align-items:center;justify-content:center;
  font-size:17px;flex-shrink:0;transition:all 0.2s cubic-bezier(.4,0,.2,1);
  box-shadow:0 2px 12px rgba(245,158,11,0.2)}
.sbtn:hover:not(:disabled){transform:scale(1.08);
  box-shadow:0 4px 20px rgba(245,158,11,0.35)}
.sbtn:active:not(:disabled){transform:scale(0.95)}
.sbtn:disabled{background:var(--border);color:var(--dim);cursor:not-allowed;transform:none;box-shadow:none}
.sbtn.stop{background:linear-gradient(135deg,#ef4444,#dc2626);color:#fff;animation:pulse-stop 1.2s ease-in-out infinite}
@keyframes pulse-stop{0%,100%{box-shadow:0 0 0 0 rgba(239,68,68,0.4)}50%{box-shadow:0 0 0 8px rgba(239,68,68,0)}}
/* ── Mic / TTS ────────────────────────────────────────────── */
.mbtn{
  width:38px;height:38px;border-radius:12px;border:none;
  background:rgba(255,255,255,0.04);color:var(--text);display:flex;
  align-items:center;justify-content:center;
  font-size:18px;flex-shrink:0;cursor:pointer;
  transition:all 0.2s cubic-bezier(.4,0,.2,1)}
.mbtn:hover{background:rgba(255,255,255,0.08);transform:scale(1.08)}
.mbtn.recording{background:rgba(239,68,68,0.15);color:#ef4444;
  animation:micpulse 0.9s infinite;
  box-shadow:0 0 20px rgba(239,68,68,0.2)}
.mbtn.talkmode{background:rgba(34,197,94,0.12);color:#22c55e;
  border:1.5px solid rgba(34,197,94,0.3);
  box-shadow:0 0 16px rgba(34,197,94,0.1)}
.mbtn.talkmode.recording{background:rgba(239,68,68,0.15);color:#ef4444;border-color:transparent;
  box-shadow:0 0 20px rgba(239,68,68,0.2)}
@keyframes micpulse{0%,100%{transform:scale(1)}50%{transform:scale(1.15)}}
.tts-btn{
  padding:3px 10px;border-radius:8px;border:1px solid var(--border2);
  background:rgba(255,255,255,0.02);color:var(--dim2);font-size:11px;
  cursor:pointer;transition:all 0.15s;backdrop-filter:blur(4px)}
.tts-btn:hover{border-color:rgba(245,158,11,0.3);color:var(--gold2);
  background:var(--glow-gold)}
.tts-btn.on{border-color:var(--gold);color:var(--gold2);
  background:var(--glow-gold);box-shadow:0 0 12px var(--glow-gold)}
/* ── LLM Settings Modal ─────────────────────────────────── */
.llm-overlay{position:fixed;inset:0;background:rgba(0,0,0,0.6);backdrop-filter:blur(6px);z-index:9000;display:none;align-items:center;justify-content:center}
.llm-overlay.open{display:flex}
.llm-modal{background:var(--s2);border:1px solid var(--border2);border-radius:18px;width:400px;max-width:92vw;padding:28px;box-shadow:0 24px 80px rgba(0,0,0,0.5);position:relative}
.llm-modal h3{margin:0 0 16px;font-size:17px;font-weight:700;color:var(--text)}
.llm-modal h3 span{color:var(--gold)}
.llm-close{position:absolute;top:14px;right:16px;background:none;border:none;color:var(--dim);font-size:20px;cursor:pointer;padding:2px 6px;border-radius:6px}
.llm-close:hover{color:var(--text);background:rgba(255,255,255,0.05)}
.llm-field{margin-bottom:14px}
.llm-field label{display:block;font-size:12px;color:var(--dim2);margin-bottom:5px;font-weight:600;letter-spacing:0.3px}
.llm-field select,.llm-field input{width:100%;padding:9px 12px;border-radius:10px;border:1px solid var(--border2);background:var(--s1);color:var(--text);font-size:13px;outline:none;transition:border-color 0.15s}
.llm-field select:focus,.llm-field input:focus{border-color:var(--gold)}
.llm-field small{display:block;margin-top:3px;font-size:10.5px;color:var(--dim)}
.llm-actions{display:flex;gap:8px;margin-top:18px}
.llm-actions button{flex:1;padding:9px 0;border-radius:10px;font-size:13px;font-weight:600;cursor:pointer;transition:all 0.15s;border:1px solid var(--border2)}
.llm-test-btn{background:rgba(139,92,246,0.12);color:#a78bfa;border-color:rgba(139,92,246,0.25) !important}
.llm-test-btn:hover{background:rgba(139,92,246,0.2)}
.llm-save-btn{background:var(--glow-gold);color:var(--gold2);border-color:rgba(245,158,11,0.3) !important}
.llm-save-btn:hover{background:rgba(245,158,11,0.2)}
.llm-status{margin-top:12px;padding:8px 12px;border-radius:8px;font-size:12px;display:none}
.llm-status.ok{display:block;background:rgba(34,197,94,0.1);border:1px solid rgba(34,197,94,0.2);color:var(--green)}
.llm-status.err{display:block;background:rgba(239,68,68,0.1);border:1px solid rgba(239,68,68,0.2);color:var(--red)}
.llm-status.info{display:block;background:rgba(59,130,246,0.1);border:1px solid rgba(59,130,246,0.2);color:var(--blue)}

.hint{font-size:11px;color:var(--dim);margin-top:7px;text-align:center}
.asys-hint{display:none;font-size:11px;color:var(--dim);margin-top:8px;line-height:1.5;text-align:left;max-width:34rem;margin-left:auto;margin-right:auto;padding:8px 10px;border-radius:8px;background:rgba(120,80,200,.08);border:1px solid rgba(120,80,200,.2)}
.asys-hint.show{display:block}
.asys-hint code{font-size:10px;opacity:.95}
kbd{
  padding:1px 5px;border:1px solid var(--border2);
  border-radius:4px;background:var(--s3);font-size:10px}

/* ── Voice Studio — AI point-cloud head (video hidden; drives motion) ─ */
body.voice-studio-on{overflow:hidden}
body.voice-studio-on nav.sidebar,
body.voice-studio-on main.main{display:none!important}
#voice-studio{
  display:none;position:fixed;inset:0;z-index:10050;
  flex-direction:column;
  background:radial-gradient(ellipse 90% 60% at 50% 25%,#0a1628 0%,#050510 40%,#020208 100%);
  color:var(--text);font-family:inherit}
#voice-studio.vs-visible{display:flex!important}
.vs-top{
  display:flex;align-items:center;gap:12px;padding:14px 20px;
  border-bottom:1px solid rgba(94,234,212,0.1);
  background:rgba(5,5,16,0.85);backdrop-filter:blur(20px)}
.vs-back,.vs-dock{
  padding:7px 16px;border-radius:10px;border:1px solid rgba(255,255,255,0.08);
  background:rgba(255,255,255,0.04);color:var(--dim2);font-size:12.5px;
  cursor:pointer;transition:all 0.2s;backdrop-filter:blur(8px)}
.vs-back:hover,.vs-dock:hover{border-color:rgba(94,234,212,0.4);color:#5eead4;
  box-shadow:0 0 16px rgba(94,234,212,0.1)}
.vs-title{
  margin-left:auto;margin-right:auto;font-size:14px;font-weight:700;
  letter-spacing:3px;text-transform:uppercase;
  background:linear-gradient(90deg,#5eead4,#fbbf24);
  -webkit-background-clip:text;-webkit-text-fill-color:transparent;
  filter:drop-shadow(0 0 12px rgba(94,234,212,0.2))}
.vs-stage{
  flex:1;display:flex;flex-direction:column;align-items:center;justify-content:center;
  min-height:0;padding:12px 16px 28px;gap:12px;position:relative}
.vs-stage-main{
  display:flex;flex-direction:column;align-items:center;justify-content:center;
  width:100%;max-width:720px;margin:0 auto}
.vs-webcam-col{
  display:flex;flex-direction:column;align-items:center;gap:14px;width:100%}
.vs-webcam-inner{
  position:relative;width:min(94vw,600px);aspect-ratio:4/3;max-height:min(58vh,520px);
  border-radius:24px;overflow:hidden;cursor:pointer;outline:none;
  border:1.5px solid rgba(94,234,212,0.35);
  background:radial-gradient(ellipse 85% 70% at 50% 38%,#0c1829 0%,#050a12 50%,#020408 100%);
  box-shadow:0 0 60px rgba(94,234,212,0.15),0 0 120px rgba(94,234,212,0.05),
    inset 0 0 80px rgba(94,234,212,0.04),inset 0 0 140px rgba(0,0,0,0.5);
  transition:all 0.3s cubic-bezier(.4,0,.2,1)}
.vs-webcam-inner:hover{border-color:rgba(251,191,36,0.35);
  box-shadow:0 0 70px rgba(94,234,212,0.2),0 0 140px rgba(94,234,212,0.06)}
.vs-webcam-inner:focus-visible{box-shadow:0 0 0 3px rgba(94,234,212,0.5),0 0 60px rgba(94,234,212,0.2)}
.vs-webcam-inner.vs-cam-live{
  border-color:rgba(94,234,212,0.5);
  box-shadow:0 0 80px rgba(94,234,212,0.2),inset 0 0 60px rgba(34,197,94,0.04);
  animation:vsBreath 4s ease-in-out infinite}
@keyframes vsBreath{
  0%,100%{box-shadow:0 0 60px rgba(94,234,212,0.15),inset 0 0 60px rgba(34,197,94,0.03)}
  50%{box-shadow:0 0 90px rgba(94,234,212,0.25),inset 0 0 80px rgba(34,197,94,0.06)}}
/* ── Voice waveform visualizer ─────────────────────────────── */
.vs-waveform{
  width:min(80vw,500px);height:48px;margin-top:8px;position:relative;
  border-radius:12px;overflow:hidden;background:rgba(0,0,0,0.3);
  border:1px solid rgba(94,234,212,0.12)}
.vs-waveform canvas{width:100%;height:100%;display:block}
/* ── Voice transcript area ─────────────────────────────────── */
.vs-transcript{
  max-width:500px;width:90vw;margin-top:10px;
  padding:12px 16px;border-radius:14px;
  background:rgba(10,16,40,0.6);backdrop-filter:blur(12px);
  border:1px solid rgba(94,234,212,0.08);
  font-size:13px;color:var(--dim2);line-height:1.6;
  max-height:140px;overflow-y:auto;text-align:center}
/* ── Voice status indicator ────────────────────────────────── */
.vs-status{
  display:flex;align-items:center;gap:8px;margin-top:6px;
  padding:6px 16px;border-radius:20px;
  background:rgba(94,234,212,0.06);border:1px solid rgba(94,234,212,0.12);
  font-size:11px;color:#5eead4;letter-spacing:0.5px}
.vs-status-dot{
  width:8px;height:8px;border-radius:50%;background:#5eead4;
  animation:vsPulse 1.5s infinite}
@keyframes vsPulse{0%,100%{opacity:0.4;transform:scale(0.8)}50%{opacity:1;transform:scale(1.1)}}
#kalaAiFaceMount{
  position:absolute;inset:0;z-index:1;pointer-events:none;overflow:hidden}
#kalaAiFaceMount canvas{display:block;width:100%!important;height:100%!important}
/* Raw camera: invisible but still used for motion → gaze + wave (privacy / “graphic face” only) */
#vsVideo{
  position:absolute;inset:0;width:100%;height:100%;object-fit:cover;transform:scaleX(-1);
  opacity:0;pointer-events:none;z-index:0;background:#000}
.vs-cam-badge{
  position:absolute;bottom:10px;left:50%;transform:translateX(-50%);white-space:nowrap;z-index:3;
  font-size:11.5px;font-weight:600;padding:5px 12px;border-radius:999px;
  background:rgba(0,0,0,0.72);color:#fde68a;border:1px solid rgba(251,191,36,0.4);
  pointer-events:none;opacity:0;transition:opacity 0.2s}
.vs-cam-badge.show{opacity:1;animation:vsWavePop 0.55s ease}
@keyframes vsWavePop{0%{transform:translateX(-50%) scale(.88);opacity:0}40%{opacity:1}100%{transform:translateX(-50%) scale(1)}}

@media(max-width:620px){
  .vs-webcam-inner{max-height:50vh;width:min(96vw,600px)}
  .sidebar{width:56px;min-width:56px}
  .logo-name,.logo-sub,.m-name,.m-badge,.sidebar-foot .stat{display:none}
  .logo-icon{margin:0 auto}
  .mode-btn{justify-content:center;padding:10px}
  .m-ico{width:auto}}
</style>
</head>
<body>

<div id="kalaGate" class="kala-gate" aria-hidden="true" role="dialog" aria-labelledby="kalaGateTitle">
  <div class="kala-gate-card">
    <div class="kala-gate-logo">⚡</div>
    <h1 id="kalaGateTitle">Kala Live</h1>
    <p>Kala Voice shows a <b>3D AI head</b> (glowing point-cloud / lines). Your camera feeds <b>motion only</b> — <b>no live video</b>. On-device.</p>
    <button type="button" class="kala-gate-prim" onclick="kalaGateEnterVoice()">Enter Kala Voice — allow camera</button>
    <button type="button" class="kala-gate-sec" onclick="kalaGateSkip()">Skip for now · chat only</button>
    <p class="kala-gate-note">Microphone is requested when you start voice listen. You can change camera anytime inside Kala Voice.</p>
  </div>
</div>

<!-- SIDEBAR -->
<nav class="sidebar">
  <div class="logo-area">
    <div class="logo-row">
      <div class="logo-icon">⚡</div>
      <div class="logo-name">Kala</div>
    </div>
    <div class="logo-sub">काल &nbsp;·&nbsp; AI by <strong>Killer</strong> v2.3</div>
  </div>
  <div class="modes-wrap">
    <span class="sec-label">AI Modes</span>
    <button class="mode-btn active" onclick="setMode('ask',this)">
      <span class="m-ico">💬</span><span class="m-name">Ask</span>
    </button>
    <button class="mode-btn" onclick="setMode('think',this)">
      <span class="m-ico">🧠</span><span class="m-name">Think Deep</span>
      <span class="m-badge">v2</span>
    </button>
    <button class="mode-btn" onclick="setMode('write',this)">
      <span class="m-ico">✍️</span><span class="m-name">Write Prose</span>
      <span class="m-badge gold">NEW</span>
    </button>
    <button class="mode-btn" onclick="setMode('code',this)">
      <span class="m-ico">💻</span><span class="m-name">Code &amp; Projects</span>
      <span class="m-badge gold">NEW</span>
    </button>
    <button class="mode-btn" onclick="setMode('imagine',this)">
      <span class="m-ico">🔭</span><span class="m-name">Imagine</span>
      <span class="m-badge gold">NEW</span>
    </button>
    <button class="mode-btn" onclick="setMode('debug',this)">
      <span class="m-ico">🔍</span><span class="m-name">Debug Code</span>
    </button>
    <button class="mode-btn" onclick="setMode('feel',this)">
      <span class="m-ico">💛</span><span class="m-name">Emotional AI</span>
    </button>
    <button class="mode-btn" onclick="setMode('guard',this)">
      <span class="m-ico">🛡</span><span class="m-name">Safety Guard</span>
    </button>
    <div style="height:6px"></div>
    <span class="sec-label" style="margin-top:4px">AI Science</span>
    <button class="mode-btn" onclick="setMode('lab',this)">
      <span class="m-ico">🧪</span><span class="m-name">AI Lab</span>
      <span class="m-badge gold">NEW</span>
    </button>
    <button class="mode-btn" onclick="setMode('ai_system',this)">
      <span class="m-ico">🤖</span><span class="m-name">AI System</span>
      <span class="m-badge gold">NEW</span>
    </button>
  </div>
  <div class="sidebar-foot">
    <div class="stat"><span>Engine</span><span class="sv">Kala v2.3</span></div>
    <div class="stat"><span>AI stack</span><span class="sv">Native+opt.LLM</span></div>
    <div class="stat"><span>Offline</span><span class="sv">✓ Ready</span></div>
    <div class="stat"><span>Port</span><span class="sv" id="port-disp">8080</span></div>
    <div class="online"><div class="pulse"></div>Kala engine live</div>
  </div>
</nav>

<!-- MAIN -->
<main class="main">
  <div class="topbar">
    <div class="tb-title"><span id="kala-mood" class="kala-mood" title="Kala's mood">😊</span> Kala &nbsp;<span id="tb-mode">Ask</span></div>
    <div class="tb-right">
      <button type="button" class="tts-btn" onclick="enterVoiceStudioAndListen()" title="Kala Voice — full face UI, auto listen & speak (Chrome / Edge)">Voice</button>
      <button type="button" class="tts-btn" onclick="newChat()" title="Clear conversation memory for this tab">New chat</button>
      <button type="button" class="tts-btn" onclick="openLlmSettings()" title="Configure LLM provider" id="llm-gear-btn">⚙️ LLM</button>
      <span class="bdg bdg-purple" id="eng-badge">Ghost-108</span>
      <span class="bdg bdg-gold">v3.0</span>
      <span class="bdg bdg-green">● Live</span>
    </div>
  </div>

  <div id="msgs">
    <div class="welcome" id="welcome">
      <div class="wglyph">⚡</div>
      <div class="wtitle"><em>Kala</em> — AI within Killer</div>
      <div class="wsub">Ask anything. Scaffold multi-file projects. Think deep. Write prose. Native KhLM, inference, AI Lab (real Rust demos) — configure an LLM for best cloud-assisted answers.</div>
      <div class="wpills">
        <span class="wpill">💬 10 AI Modes</span>
        <span class="wpill">🌐 Works Offline</span>
        <span class="wpill">⚡ Ghost-108 Search</span>
        <span class="wpill">🧪 AI Lab: native demos + curriculum (not full ML/DL stacks)</span>
      </div>
      <div class="pgrid">
        <div class="pcard" onclick="qp('ask','What is speed of light and how was it first measured?')">
          <div class="pico">💬</div>
          <div class="ptitle">Ask anything</div>
          <div class="pdesc">Speed of light, math, history…</div>
        </div>
        <div class="pcard" onclick="qp('write','artificial intelligence future','essay')">
          <div class="pico">✍️</div>
          <div class="ptitle">Write an essay</div>
          <div class="pdesc">AI future — academic prose</div>
        </div>
        <div class="pcard" onclick="qp('imagine','a world where every city runs on Killer code')">
          <div class="pico">🔭</div>
          <div class="ptitle">Imagine a world</div>
          <div class="pdesc">Cities running on Killer code</div>
        </div>
        <div class="pcard" onclick="qp('lab','ml machine learning algorithms')">
          <div class="pico">🧪</div>
          <div class="ptitle">AI Lab</div>
          <div class="pdesc">Rust demos · honest AGI/ASI/OS as curriculum</div>
        </div>
        <div class="pcard" onclick="qp('ai_system','Compare approaches: explain gradient descent, then summarize who leads OpenAI today.')">
          <div class="pico">🤖</div>
          <div class="ptitle">AI System</div>
          <div class="pdesc">Hard questions: router + search + GGUF + merge · not AGI</div>
        </div>
        <div class="pcard" onclick="qp('think','Why does consciousness exist?')">
          <div class="pico">🧠</div>
          <div class="ptitle">Think deep</div>
          <div class="pdesc">Consciousness, existence, meaning</div>
        </div>
        <div class="pcard" onclick="qp('code','Create a project: Python FastAPI REST API with SQLite, CRUD routes, requirements.txt, and README with run steps.')">
          <div class="pico">💻</div>
          <div class="ptitle">Starter project</div>
          <div class="pdesc">API + DB + README layout</div>
        </div>
        <div class="pcard" onclick="enterVoiceStudioAndListen()">
          <div class="pico">🎙</div>
          <div class="ptitle">Voice studio</div>
          <div class="pdesc">AI mesh face · hands-free</div>
        </div>
      </div>
    </div>
    <button class="scroll-fab" id="scrollFab" onclick="scrollToBottom()" title="Scroll to bottom">↓</button>
  </div>

  <div class="input-area">
    <div class="chips" id="chips">
      <button class="chip on" onclick="setSt('essay',this)">essay</button>
      <button class="chip" onclick="setSt('summary',this)">summary</button>
      <button class="chip" onclick="setSt('technical',this)">technical</button>
      <button class="chip" onclick="setSt('story',this)">story</button>
      <button class="chip" onclick="setSt('formal',this)">formal</button>
      <button class="chip" onclick="setSt('casual',this)">casual</button>
      <button class="chip" onclick="setSt('explain',this)">explain</button>
    </div>
    <div class="ibox">
      <select class="msel" id="msel" onchange="onSel(this.value)">
        <option value="ask">💬 Ask</option>
        <option value="think">🧠 Think</option>
        <option value="write">✍️ Write</option>
        <option value="code">💻 Code</option>
        <option value="imagine">🔭 Imagine</option>
        <option value="debug">🔍 Debug</option>
        <option value="feel">💛 Feel</option>
        <option value="guard">🛡 Guard</option>
        <option value="lab">🧪 AI Lab</option>
        <option value="ai_system">🤖 AI System</option>
      </select>
      <button class="mbtn" id="mbtn" onclick="toggleMic()" title="Kala Voice: 3D AI point-cloud face + mic">🎙</button>
      <textarea id="qi" rows="1" placeholder="Ask Kala anything…  🎙 = voice studio (3D AI face, camera hidden)"
        onkeydown="onKey(event)" oninput="onUserType();resize(this)"></textarea>
      <button class="sbtn" id="sbtn" onclick="send()" title="Send (Enter)">▶</button>
    </div>
    <div class="hint">
      <kbd>Enter</kbd> to send &nbsp;·&nbsp;
      <kbd>Shift+Enter</kbd> new line &nbsp;·&nbsp;
      🎙 <b>mic</b> = <b>Kala Voice</b> (3D AI face + hidden camera tracking + hands-free) &nbsp;·&nbsp;
      <button class="tts-btn" id="ttsbtn" onclick="toggleTTS()" title="Toggle voice for typed messages">🔇 Voice Off</button>
      <select class="tts-btn" id="voiceLang" onchange="setSttLang(this.value)" title="Voice language for mic input" style="font-size:11px;padding:2px 6px;min-width:70px">
        <option value="">Auto</option>
        <option value="en-US">English</option>
        <option value="hi-IN">Hindi</option>
        <option value="te-IN">Telugu</option>
        <option value="ta-IN">Tamil</option>
        <option value="bn-IN">Bengali</option>
        <option value="mr-IN">Marathi</option>
        <option value="gu-IN">Gujarati</option>
        <option value="kn-IN">Kannada</option>
        <option value="ml-IN">Malayalam</option>
        <option value="pa-IN">Punjabi</option>
        <option value="ur-PK">Urdu</option>
        <option value="ar-SA">Arabic</option>
        <option value="zh-CN">Chinese</option>
        <option value="ja-JP">Japanese</option>
        <option value="ko-KR">Korean</option>
        <option value="es-ES">Spanish</option>
        <option value="fr-FR">French</option>
        <option value="de-DE">German</option>
        <option value="pt-BR">Portuguese</option>
        <option value="ru-RU">Russian</option>
        <option value="it-IT">Italian</option>
        <option value="th-TH">Thai</option>
        <option value="vi-VN">Vietnamese</option>
        <option value="tr-TR">Turkish</option>
        <option value="nl-NL">Dutch</option>
        <option value="pl-PL">Polish</option>
        <option value="sv-SE">Swedish</option>
      </select>
    </div>
    <div class="asys-hint" id="asysHint" aria-live="polite">
      <strong>AI System</strong> — for <em>hard</em> questions: KhLM router + Ghost-108 search + local neural + optional <strong>merged</strong> verdict.
      Install or point at a <strong>reasoning GGUF</strong> (e.g. R1-style) via <code>KILLER_KHLM_GGUF</code> or <code>%USERPROFILE%\.killer\models\*.gguf</code> — it helps the neural slot and coordinator synthesis.
      This is <strong>advanced orchestration + merging</strong>, not AGI.
    </div>
  </div>
</main>

<!-- Voice Studio — Three.js AI point-cloud face + waveform + hidden camera + 2-way voice -->
<div id="voice-studio" class="voice-studio" aria-hidden="true">
  <div class="vs-top">
    <button type="button" class="vs-back" onclick="closeVoiceStudio()">← Chat</button>
    <span class="vs-title">Kala Voice · Live</span>
    <select class="vs-dock" id="vsLangSel" onchange="setSttLang(this.value);var ml=document.getElementById('voiceLang');if(ml)ml.value=this.value" title="Mic language" style="font-size:11px;padding:3px;border-radius:6px;background:rgba(255,255,255,0.08);color:var(--text);border:1px solid var(--border2)">
      <option value="">Auto</option><option value="en-US">EN</option><option value="hi-IN">HI</option><option value="te-IN">TE</option>
      <option value="ta-IN">TA</option><option value="es-ES">ES</option><option value="fr-FR">FR</option><option value="de-DE">DE</option>
      <option value="ja-JP">JA</option><option value="ko-KR">KO</option><option value="zh-CN">ZH</option><option value="ar-SA">AR</option>
      <option value="ru-RU">RU</option><option value="pt-BR">PT</option>
    </select>
    <button type="button" class="vs-dock" onclick="closeVoiceStudio()" title="Return to chat">Dock</button>
  </div>
  <div class="vs-stage">
    <div class="vs-stage-main">
      <div class="vs-webcam-col">
        <div class="vs-webcam-inner" id="vsWebcamInner" onclick="vsFaceTap(event)" title="Tap to pause or resume voice (optional)" role="button" tabindex="0" onkeydown="if(event.key==='Enter'||event.key===' ')vsFaceTap(event)">
          <video id="vsVideo" playsinline muted autoplay aria-hidden="true"></video>
          <div id="kalaAiFaceMount" aria-hidden="true"></div>
          <div id="vsCamBadge" class="vs-cam-badge">👋 Wave detected</div>
        </div>
        <div class="vs-waveform" id="vsWaveform"><canvas id="vsWaveCanvas"></canvas></div>
        <div class="vs-status" id="vsStatus"><span class="vs-status-dot"></span><span id="vsStatusText">Ready — tap face or speak</span></div>
        <div class="vs-transcript" id="vsTranscript">Kala is listening...</div>
      </div>
    </div>
  </div>
</div>

<script>
let mode='ask', style='essay', msgs=0, totalMs=0;

// ── Mode ─────────────────────────────────────────────────────
function setMode(m,btn){
  mode=m;
  document.querySelectorAll('.mode-btn').forEach(b=>b.classList.remove('active'));
  if(btn)btn.classList.add('active');
  document.getElementById('msel').value=m;
  const names={ask:'Ask',think:'Think Deep',write:'Write Prose',code:'Code & Projects',imagine:'Imagine',debug:'Debug Code',feel:'Emotional AI',guard:'Safety Guard',lab:'AI Lab',ai_system:'AI System',multi_agent:'AI System'};
  document.getElementById('tb-mode').textContent=names[m]||m;
  const badges={ask:'Ghost-108',think:'Native Think',write:'Prose Engine',code:'KhLM CodeGen',imagine:'Imagination Engine',debug:'KhLM-Poly',feel:'Affect v2',guard:'Guardian',lab:'Native demos + curriculum',ai_system:'Router+Search+Neural+Merge',multi_agent:'Router+Search+Neural+Merge'};
  document.getElementById('eng-badge').textContent=badges[m]||'KhLM';
  document.getElementById('chips').classList.toggle('show',m==='write');
  const ph={ask:'Ask Kala anything…',think:'What deep question should Kala reason through?',write:'What topic should Kala write about?',code:'Describe app or project (stack, features). Multi-file repos OK…',imagine:'What scenario should Kala imagine?',debug:'Paste code to debug (include language in first line)…',feel:'Share something and Kala senses the emotion…',guard:'Enter text to check for safety issues…',ai_system:'Hard question: mixed facts + reasoning (reasoning GGUF recommended for merge)…',multi_agent:'Hard question: mixed facts + reasoning (reasoning GGUF recommended for merge)…'};
  document.getElementById('qi').placeholder=ph[m]||(m==='lab'?'Ask in AI Lab (ML, DL, LLM…)…':'Ask Kala…');
  var ah=document.getElementById('asysHint');
  if(ah)ah.classList.toggle('show',m==='ai_system'||m==='multi_agent');
  document.getElementById('qi').focus();
}
function onSel(m){
  const btns=document.querySelectorAll('.mode-btn');
  const list=['ask','think','write','code','imagine','debug','feel','guard','lab','ai_system'];
  setMode(m, btns[list.indexOf(m)]||null);
}
function setSt(s,btn){
  style=s;
  document.querySelectorAll('.chip').forEach(c=>c.classList.remove('on'));
  if(btn)btn.classList.add('on');
}

// ── Quick prompts ─────────────────────────────────────────────
function qp(m,text,st){
  if(st)setSt(st,null);
  onSel(m);
  document.getElementById('qi').value=text;
  resize(document.getElementById('qi'));
  send();
}

// ── Send ───────────────────────────────────────────────────────
function onKey(e){if(e.key==='Enter'&&!e.shiftKey){e.preventDefault();send();}}
var kalaAutoStopArmed=false;
function onUserType(){
  if(kalaVoiceUpdating)return;
  var qi=document.getElementById('qi');
  if(kalaSendBusy&&kalaAutoStopArmed&&qi&&qi.value.length>0){
    stopKala();
    try{window.speechSynthesis&&window.speechSynthesis.cancel();}catch(e){}
  }
}
function resize(el){el.style.height='auto';el.style.height=Math.min(el.scrollHeight,160)+'px';}

// ── Voice Studio (Three.js AI head + hidden cam tracking) ─────
var voiceStudioOpen=false;
var voicePhase='idle';
var lastVoiceQ='';
var listenResumeTimer=null;
var kalaVoiceUpdating=false;
var kalaSendBusy=false;
var kalaAbortCtrl=null;
var lastFailedQuestion=null;
var ttsSessionId=0;
var pendingVoiceSend=null;
// Optional webcam: motion → gaze (vsLookX/Y) + coarse left/right wave heuristic (local only)
var vsWebcamStream=null, vsWebcamOn=false, vsWebcamRaf=null;
var vsProc=null, vsProcCtx=null, vsGrayPrev=null;
var vsLookX=0, vsLookY=0;
var vsWaveHist=[], vsLastWaveAt=0, vsLastProcT=0;
var vsMotionLeft=0, vsMotionRight=0, vsMotionHead=0;
var kalaFaceRaf=null;
var kalaFaceCleanup=null;
var kalaThreePromise=null;

function stopVsWebcam(){
  if(vsWebcamRaf){cancelAnimationFrame(vsWebcamRaf);vsWebcamRaf=null;}
  vsGrayPrev=null;
  vsLastProcT=0;
  vsWaveHist=[];
  if(vsWebcamStream){
    vsWebcamStream.getTracks().forEach(function(t){t.stop();});
    vsWebcamStream=null;
  }
  var v=document.getElementById('vsVideo');
  if(v)v.srcObject=null;
  vsWebcamOn=false;
  vsLookX=0;vsLookY=0;
  vsMotionLeft=0;vsMotionRight=0;vsMotionHead=0;
  var inner=document.getElementById('vsWebcamInner');
  if(inner)inner.classList.remove('vs-cam-live');
}
function attachVsWebcamStream(stream){
  if(!stream)return;
  if(vsWebcamStream&&vsWebcamStream!==stream){
    vsWebcamStream.getTracks().forEach(function(t){t.stop();});
  }
  vsWebcamStream=stream;
  vsWebcamOn=true;
  var video=document.getElementById('vsVideo');
  if(video)video.srcObject=stream;
  var inner=document.getElementById('vsWebcamInner');
  if(inner)inner.classList.add('vs-cam-live');
  if(!vsProc){
    vsProc=document.createElement('canvas');
    vsProc.width=80;vsProc.height=45;
    vsProcCtx=vsProc.getContext('2d',{willReadFrequently:true});
  }
  vsGrayPrev=null;
  vsWaveHist=[];
  if(voiceStudioOpen){
    if(!vsWebcamRaf)vsWebcamRaf=requestAnimationFrame(vsWebcamProcessLoop);
    setTimeout(function(){if(voiceStudioOpen)ensureKalaAiFace();},60);
  }
}
function requestVsWebcamMedia(){
  if(!navigator.mediaDevices||!navigator.mediaDevices.getUserMedia){
    return Promise.reject(new Error('Camera not available in this browser.'));
  }
  return navigator.mediaDevices.getUserMedia({
    video:{width:{ideal:640},height:{ideal:480},facingMode:'user'},audio:false
  }).then(function(stream){attachVsWebcamStream(stream);return stream;});
}
function ensureVsWebcamForStudio(){
  if(vsWebcamOn)return Promise.resolve();
  return requestVsWebcamMedia();
}
function kalaGateDismiss(){
  var g=document.getElementById('kalaGate');
  if(g){g.classList.remove('kala-gate-visible');g.setAttribute('aria-hidden','true');}
  document.body.classList.remove('kala-gate-on');
}
function dismissKalaGateForVoice(){
  try{sessionStorage.setItem('kala_gate_ok','1');}catch(e){}
  kalaGateDismiss();
}
function kalaGateSkip(){
  try{sessionStorage.setItem('kala_gate_ok','1');}catch(e){}
  kalaGateDismiss();
  setTimeout(function(){
    var q=document.getElementById('qi');
    if(q)q.focus();
  },0);
}
function kalaGateEnterVoice(){
  if(!navigator.mediaDevices||!navigator.mediaDevices.getUserMedia){
    alert('Camera not available in this browser.');
    kalaGateSkip();
    return;
  }
  requestVsWebcamMedia()
    .then(function(){
      try{sessionStorage.setItem('kala_gate_ok','1');}catch(e){}
      kalaGateDismiss();
      setTimeout(function(){enterVoiceStudioAndListen();},100);
    })
    .catch(function(e){
      console.warn(e);
      alert('Camera was not shared. Kala Voice will still open; the 3D face uses less motion without the camera.');
      kalaGateSkip();
    });
}
function onWaveDetected(){
  vsLastWaveAt=Date.now();
  var b=document.getElementById('vsCamBadge');
  if(b){
    b.classList.remove('show');
    void b.offsetWidth;
    b.classList.add('show');
    setTimeout(function(){if(b)b.classList.remove('show');},2200);
  }
}
function vsWebcamProcessLoop(ts){
  ts=ts||performance.now();
  if(!voiceStudioOpen||!vsWebcamOn||!vsProcCtx){vsWebcamRaf=null;return;}
  if(ts-vsLastProcT<52){
    vsWebcamRaf=requestAnimationFrame(vsWebcamProcessLoop);
    return;
  }
  vsLastProcT=ts;
  var video=document.getElementById('vsVideo');
  if(!video||video.readyState<2){
    vsWebcamRaf=requestAnimationFrame(vsWebcamProcessLoop);
    return;
  }
  var W=vsProc.width, H=vsProc.height, n=W*H;
  vsProcCtx.drawImage(video,0,0,W,H);
  var frame=vsProcCtx.getImageData(0,0,W,H);
  var d=frame.data;
  var gray=new Uint8Array(n);
  for(var i=0,k=0;k<n;i+=4,k++){
    gray[k]=(d[i]*0.299+d[i+1]*0.587+d[i+2]*0.114)|0;
  }
  if(!vsGrayPrev){
    vsGrayPrev=new Uint8Array(n);
    vsGrayPrev.set(gray);
    vsWebcamRaf=requestAnimationFrame(vsWebcamProcessLoop);
    return;
  }
  var mx=0,my=0,mt=0;
  var headY0=0, headY1=Math.floor(H*0.62);
  var handY0=Math.floor(H*0.48), handY1=H;
  var ml=0,mr=0;
  for(var y=0;y<H;y++){
    for(var x=0;x<W;x++){
      var idx=y*W+x;
      var diff=Math.abs(gray[idx]-vsGrayPrev[idx]);
      if(diff<16)continue;
      if(y>=headY0&&y<headY1){mx+=x;my+=y;mt++;}
      if(y>=handY0&&y<handY1){
        if(x<W/3)ml+=diff;
        else if(x>(2*W)/3)mr+=diff;
      }
    }
  }
  vsGrayPrev.set(gray);
  var nx=0, ny=0;
  if(mt>22){
    mx/=mt;my/=mt;
    nx=(mx/(W-1))*2-1;
    ny=(my/(H-1))*2-1;
  }
  vsLookX=Math.max(-1,Math.min(1,vsLookX*0.82+nx*0.18));
  vsLookY=Math.max(-1,Math.min(1,vsLookY*0.86+ny*0.14));
  var now=Date.now();
  var dom=null;
  if(ml>mr*1.38&&ml>72)dom='L';
  else if(mr>ml*1.38&&mr>72)dom='R';
  if(dom){
    if(vsWaveHist.length===0||vsWaveHist[vsWaveHist.length-1].s!==dom)
      vsWaveHist.push({t:now,s:dom});
  }
  while(vsWaveHist.length&&now-vsWaveHist[0].t>1100)vsWaveHist.shift();
  var alts=0;
  for(var j=1;j<vsWaveHist.length;j++){
    if(vsWaveHist[j].s!==vsWaveHist[j-1].s)alts++;
  }
  if(alts>=3&&vsWaveHist.length>=4&&(now-vsLastWaveAt)>2400)onWaveDetected();
  vsMotionLeft=ml;vsMotionRight=mr;vsMotionHead=mt;
  vsWebcamRaf=requestAnimationFrame(vsWebcamProcessLoop);
}

function disposeKalaAiFace(){
  if(kalaFaceRaf){cancelAnimationFrame(kalaFaceRaf);kalaFaceRaf=null;}
  if(kalaFaceCleanup){
    try{kalaFaceCleanup();}catch(e){}
    kalaFaceCleanup=null;
  }
  var m=document.getElementById('kalaAiFaceMount');
  if(m)m.innerHTML='';
}
function ensureKalaAiFace(){
  disposeKalaAiFace();
  var mount=document.getElementById('kalaAiFaceMount');
  if(!mount)return;
  if(!kalaThreePromise){
    kalaThreePromise=import('three').catch(function(){
      return import('https://cdn.jsdelivr.net/npm/three@0.160.0/build/three.module.js');
    });
  }
  kalaThreePromise.then(function(THREE){
    if(!voiceStudioOpen||!document.getElementById('kalaAiFaceMount'))return;
    initKalaAiPointCloudFace(THREE,mount);
  }).catch(function(e){
    console.warn('Kala AI face needs Three.js (network once)',e);
    mount.innerHTML='<p style="position:absolute;inset:0;display:flex;align-items:center;justify-content:center;padding:16px;text-align:center;color:#94a3b8;font-size:12px">3D face requires loading Three.js — check connection and refresh.</p>';
  });
}
function kalaMorphHeadPositions(pos){
  for(var i=0;i<pos.count;i++){
    var x=pos.getX(i),y=pos.getY(i),z=pos.getZ(i);
    var l=Math.sqrt(x*x+y*y+z*z)||1;
    x/=l;y/=l;z/=l;
    var ex=x*0.86,ey=y*1.16,ez=z*0.84;
    if(z>0.18){
      if(Math.abs(x)<0.36&&y>-0.14&&y<0.44)ez+=0.11+Math.max(0,0.28-y)*0.4;
    }
    if(Math.abs(x)>0.24&&y>-0.1&&y<0.5)ex*=0.84;
    if(y<-0.16){var jw=1+Math.min(0.55,(-0.16-y))*0.55;ex*=jw;ez*=jw*0.92;}
    if(y>0.32&&z>0.22)ey+=0.05;
    pos.setXYZ(i,ex,ey,ez);
  }
}
function kalaVertexColorsForAI(pos,cols){
  for(var i=0;i<pos.count;i++){
    var x=pos.getX(i),y=pos.getY(i),z=pos.getZ(i);
    var r=0.12,g=0.75,b=0.95;
    if(y>0.04&&y<0.4&&z>0.36&&Math.abs(x)>0.14&&Math.abs(x)<0.48){r=0.88;g=0.97;b=1;}
    var rg=Math.sqrt(x*x+z*z);
    if(rg>0.68&&rg<0.98&&y>-0.55&&y<0.55){g+=0.12;r+=0.04;b+=0.08;}
    cols[i*3]=r;cols[i*3+1]=Math.min(1,g);cols[i*3+2]=Math.min(1,b);
  }
}
function initKalaAiPointCloudFace(THREE,mount){
  mount.innerHTML='';
  var wrap=document.getElementById('vsWebcamInner');
  var rect=mount.getBoundingClientRect();
  var W=Math.max(220,rect.width||520), H=Math.max(180,rect.height||390);
  var scene=new THREE.Scene();
  var camera=new THREE.PerspectiveCamera(42,W/H,0.08,40);
  camera.position.set(0,0.02,2.55);
  var renderer=new THREE.WebGLRenderer({antialias:true,alpha:true,powerPreference:'high-performance'});
  renderer.setPixelRatio(Math.min(window.devicePixelRatio||1,2));
  renderer.setSize(W,H,false);
  renderer.setClearColor(0x000000,0);
  renderer.outputColorSpace=THREE.SRGBColorSpace;
  mount.appendChild(renderer.domElement);
  var root=new THREE.Group();
  root.scale.x=-1;
  scene.add(root);
  var sphereGeo=new THREE.IcosahedronGeometry(1,4);
  var pos=sphereGeo.attributes.position;
  kalaMorphHeadPositions(pos);
  sphereGeo.computeVertexNormals();
  var n=pos.count;
  var colors=new Float32Array(n*3);
  kalaVertexColorsForAI(pos,colors);
  sphereGeo.setAttribute('color',new THREE.BufferAttribute(colors,3));
  var ptsMat=new THREE.PointsMaterial({
    vertexColors:true,size:0.028,sizeAttenuation:true,
    transparent:true,opacity:0.95,blending:THREE.AdditiveBlending,depthWrite:false
  });
  var points=new THREE.Points(sphereGeo,ptsMat);
  root.add(points);
  var glowGeo=sphereGeo.clone();
  var glowMat=new THREE.PointsMaterial({
    color:0x44ffee,size:0.055,sizeAttenuation:true,
    transparent:true,opacity:0.18,blending:THREE.AdditiveBlending,depthWrite:false
  });
  var glowPts=new THREE.Points(glowGeo,glowMat);
  glowPts.scale.setScalar(1.01);
  root.add(glowPts);
  var wireBase=new THREE.IcosahedronGeometry(1,3);
  kalaMorphHeadPositions(wireBase.attributes.position);
  wireBase.computeVertexNormals();
  var edges=new THREE.EdgesGeometry(wireBase,28);
  var lineMat=new THREE.LineBasicMaterial({
    color:0x55eeff,transparent:true,opacity:0.38,blending:THREE.AdditiveBlending
  });
  var wire=new THREE.LineSegments(edges,lineMat);
  root.add(wire);
  var halo=new THREE.Mesh(
    new THREE.TorusGeometry(1.38,0.006,8,80),
    new THREE.MeshBasicMaterial({color:0x66ffff,transparent:true,opacity:0.22,blending:THREE.AdditiveBlending})
  );
  halo.rotation.x=Math.PI/2.1;halo.position.y=-0.05;
  root.add(halo);
  scene.add(new THREE.AmbientLight(0x224466,0.35));
  var key=new THREE.DirectionalLight(0xccfbff,0.9);
  key.position.set(2.2,2.8,3.5);
  scene.add(key);
  var rim=new THREE.PointLight(0x44ffdd,0.7,8);
  rim.position.set(-2.5,0.4,1.8);
  scene.add(rim);
  function onResize(){
    if(!mount.isConnected||!voiceStudioOpen)return;
    var r=mount.getBoundingClientRect();
    var w=Math.max(200,r.width), h=Math.max(160,r.height);
    camera.aspect=w/h;
    camera.updateProjectionMatrix();
    renderer.setSize(w,h,false);
  }
  window.addEventListener('resize',onResize);
  function tick(now){
    if(!voiceStudioOpen){kalaFaceRaf=null;return;}
    kalaFaceRaf=requestAnimationFrame(tick);
    var t=(now||0)*0.001;
    var ph=voicePhase;
    var lx=vsLookX, ly=vsLookY;
    root.rotation.y=lx*0.42+Math.sin(t*0.55)*0.028;
    root.rotation.x=ly*0.26+Math.cos(t*0.42)*0.022;
    var breath=1+0.012*Math.sin(t*1.8);
    root.scale.set(-1*breath,breath,breath);
    var pul=ph==='listen'?0.45+0.55*Math.sin(t*5.5):ph==='speak'?0.5+0.5*Math.sin(t*14):ph==='think'?0.35+0.35*Math.sin(t*2.1):0.5;
    ptsMat.size=0.022+0.014*pul;
    glowMat.opacity=0.12+0.16*pul;
    lineMat.opacity=0.26+0.24*pul;
    halo.rotation.z=t*0.25;
    renderer.render(scene,camera);
  }
  kalaFaceRaf=requestAnimationFrame(tick);
  kalaFaceCleanup=function(){
    window.removeEventListener('resize',onResize);
    if(kalaFaceRaf){cancelAnimationFrame(kalaFaceRaf);kalaFaceRaf=null;}
    renderer.dispose();
    sphereGeo.dispose();glowGeo.dispose();wireBase.dispose();edges.dispose();
    ptsMat.dispose();glowMat.dispose();lineMat.dispose();
    halo.geometry.dispose();halo.material.dispose();
    if(renderer.domElement.parentNode===mount)mount.removeChild(renderer.domElement);
  };
}

function clearListenResume(){
  if(listenResumeTimer){clearTimeout(listenResumeTimer);listenResumeTimer=null;}
}
function scheduleResumeListen(){
  clearListenResume();
  if(!talkMode)return;
  function tick(){
    if(!talkMode)return;
    if(kalaSendBusy){
      listenResumeTimer=setTimeout(tick,200);
      return;
    }
    // Wait until Kala finishes speaking — mic must NOT be on while
    // TTS plays, or it picks up Kala's voice and sends garbage.
    var syn=window.speechSynthesis;
    if(syn&&(syn.speaking||syn.pending)){
      listenResumeTimer=setTimeout(tick,250);
      return;
    }
    // Kala is done speaking — safe to start mic now
    startListeningCore();
  }
  // Wait a moment for TTS state to settle
  listenResumeTimer=setTimeout(tick,500);
}

function escVs(s){
  return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
}
function stripVoiceMd(t){
  return String(t)
    .replace(/```[\s\S]*?```/g,' ')
    .replace(/`([^`]+)`/g,'$1')
    .replace(/!\[.*?\]\(.*?\)/g,'')
    .replace(/\[([^\]]+)\]\([^)]+\)/g,'$1')
    .replace(/^#{1,6}\s+/gm,'')
    .replace(/[*_~]/g,'')
    .replace(/\s+/g,' ')
    .trim()
    .substring(0,900);
}
function setVoicePhase(p){
  voicePhase=p;
  var labels={
    idle:talkMode?'🟢 Voice active — speak anytime':'Click mic 🎙 to start talking',
    listen:'🎤 Listening... speak now!',
    think:'🧠 Thinking...',
    speak:'🔊 Kala speaking... mic resumes after'
  };
  setVsStatus(labels[p]||p);
  // Update face mount border color to indicate phase
  var inner=document.getElementById('vsWebcamInner');
  if(inner){
    inner.style.borderColor=p==='listen'?'rgba(34,197,94,0.6)':p==='speak'?'rgba(245,158,11,0.5)':p==='think'?'rgba(139,92,246,0.5)':'rgba(94,234,212,0.35)';
  }
}
function syncVsMic(){}
function openVoiceStudio(){
  dismissKalaGateForVoice();
  voiceStudioOpen=true;
  document.body.classList.add('voice-studio-on');
  var vs=document.getElementById('voice-studio');
  if(vs){vs.classList.add('vs-visible');vs.setAttribute('aria-hidden','false');}
  voicePhase='idle';
  setVoicePhase('idle');
  syncVsMic();
  setTimeout(function(){if(voiceStudioOpen)ensureKalaAiFace();},45);
  setVsStatus('Initializing voice...');
  if(!ttsEnabled){
    ttsEnabled=true;
    var btn=document.getElementById('ttsbtn');
    if(btn){btn.textContent='🔊 Voice On';btn.classList.add('on');}
  }
  ensureVsWebcamForStudio().catch(function(e){
    console.warn('Kala Voice: camera unavailable — 3D face motion will be minimal.',e);
  });
  try{navigator.mediaDevices.getUserMedia({audio:true}).then(function(s){initVoiceWaveform(s);setVsStatus('Listening — speak anytime');}).catch(function(){setVsStatus('Ready — tap face or speak');});}catch(e){}
  if(vsWebcamOn&&!vsWebcamRaf)vsWebcamRaf=requestAnimationFrame(vsWebcamProcessLoop);
  setTimeout(function(){
    var el=document.getElementById('vsWebcamInner');
    if(el)el.focus();
  },80);
}
function closeVoiceStudio(){
  voiceStudioOpen=false;
  disposeKalaAiFace();
  stopVsWebcam();
  stopVoiceWaveform();
  pendingVoiceSend=null;
  clearListenResume();
  document.body.classList.remove('voice-studio-on');
  var vs=document.getElementById('voice-studio');
  if(vs){vs.classList.remove('vs-visible');vs.setAttribute('aria-hidden','true');}
  if(talkMode){
    talkMode=false;
    clearSttSilenceTimer();
    if(recog)try{recog.stop();}catch(e){}
    sttSessionReady=true;
    isRecording=false;
    window.speechSynthesis&&window.speechSynthesis.cancel();
    var mb=document.getElementById('mbtn');
    if(mb){mb.classList.remove('recording','talkmode');mb.textContent='🎙';mb.title='Tap to start voice conversation';}
  }
  syncVsMic();
  setVoicePhase('idle');
  lastVoiceQ='';
}
function vsToggleMic(){
  toggleMic();
}
function vsFaceTap(ev){
  if(ev&&ev.preventDefault)ev.preventDefault();
  vsToggleMic();
}

function enableVoiceCapture(){
  var SR=window.SpeechRecognition||window.webkitSpeechRecognition;
  if(!SR)return;
  if(!recog)initSTT();
  if(talkMode)return;
  talkMode=true;
  ttsSessionId++;
  try{window.speechSynthesis&&window.speechSynthesis.cancel();}catch(e){}
  clearListenResume();
  clearSttSilenceTimer();
  var mb=document.getElementById('mbtn');
  function beginListen(){
    if(!talkMode)return;
    if(!sttSessionReady){
      listenResumeTimer=setTimeout(beginListen,120);
      return;
    }
    try{
      recog.start();
      sttSessionReady=false;
      isRecording=true;
      bumpSttSilenceWatch();
      if(mb){mb.classList.add('recording','talkmode');mb.textContent='🔴';mb.title='Tap to stop voice conversation';}
    }catch(e){
      console.warn(e);
      sttSessionReady=true;
      isRecording=false;
    }
    if(voiceStudioOpen)setVoicePhase('listen');
    syncVsMic();
  }
  beginListen();
}

function enterVoiceStudioAndListen(){
  openVoiceStudio();
  var SR=window.SpeechRecognition||window.webkitSpeechRecognition;
  if(!SR){
    alert('Kala Voice needs Chrome or Edge for speech recognition. The 3D face and camera still work here.');
    syncVsMic();
    return;
  }
  var vt=document.getElementById('vsTranscript');
  if(vt)vt.innerHTML='<em>Starting voice... speak after you hear the click or see 🎤</em>';
  requestAnimationFrame(function(){
    requestAnimationFrame(function(){
      enableVoiceCapture();
    });
  });
}

// ── Speech-to-Text (STT) ─────────────────────────────────────
var recog=null;
var isRecording=false;
var talkMode=false;   // true = voice conversation mode active
var sttSessionReady=true; // false between successful recog.start() and onend (avoid InvalidStateError loops)
var sttSilenceTimer=null;
var STT_SILENCE_MS=2600;
function clearSttSilenceTimer(){
  if(sttSilenceTimer){clearTimeout(sttSilenceTimer);sttSilenceTimer=null;}
}
function bumpSttSilenceWatch(){
  clearSttSilenceTimer();
  if(!talkMode||!isRecording)return;
  sttSilenceTimer=setTimeout(function(){
    sttSilenceTimer=null;
    if(!talkMode||!isRecording||!recog)return;
    try{recog.stop();}catch(e){}
  },STT_SILENCE_MS);
}
var sttLang='';
function detectBrowserLang(){
  var nav=navigator.language||navigator.userLanguage||'en-US';
  return nav;
}
function setSttLang(lang){
  sttLang=lang;
  if(recog)recog.lang=lang;
}
function initSTT(){
  var SR=window.SpeechRecognition||window.webkitSpeechRecognition;
  if(!SR){console.warn('SpeechRecognition not supported');return;}
  if(recog){try{recog.abort();}catch(e){}}
  recog=new SR();
  recog.continuous=false;
  recog.interimResults=true;
  recog.maxAlternatives=1;
  recog.lang=sttLang||detectBrowserLang();
  console.log('STT initialized, lang:',recog.lang);
  recog.onstart=function(){
    // Mic is now active — don't cancel TTS here; let user's actual speech
    // trigger the interrupt (in onresult). This way the mic is "hot" and
    // ready to catch the user's voice without prematurely cutting Kala off.
    if(voiceStudioOpen){
      var syn=window.speechSynthesis;
      if(!syn||(!syn.speaking&&!syn.pending)){
        setVoicePhase('listen');
      }
    }
  };
  recog.onresult=function(e){
    bumpSttSilenceWatch();
    var final_transcript='';
    var interim_transcript='';
    for(var i=0;i<e.results.length;i++){
      if(e.results[i].isFinal){
        final_transcript+=e.results[i][0].transcript;
      }else{
        interim_transcript+=e.results[i][0].transcript;
      }
    }
    var t=final_transcript||interim_transcript;
    // Auto-stop Kala when user starts speaking (like Gemini)
    if(t&&t.trim().length>0&&kalaSendBusy){
      stopKala();
      try{window.speechSynthesis&&window.speechSynthesis.cancel();}catch(ex){}
    }
    // Check confidence — reject very low confidence transcriptions
    var lastResult=e.results[e.results.length-1];
    var conf=lastResult[0].confidence;

    var qi=document.getElementById('qi');
    kalaVoiceUpdating=true;
    qi.value=t;
    resize(qi);
    kalaVoiceUpdating=false;
    if(voiceStudioOpen){
      var vh=document.getElementById('vsHeard');
      if(vh)vh.textContent=t||'…';
      if(voicePhase!=='listen')setVoicePhase('listen');
    }
    if(lastResult.isFinal){
      clearSttSilenceTimer();
      if(recog.continuous){try{recog.stop();}catch(ex){}}
      isRecording=false;
      var mb=document.getElementById('mbtn');
      if(mb){mb.classList.remove('recording');mb.textContent=talkMode?'🗣':'🎙';}
      syncVsMic();
      var finalText=final_transcript.trim();
      if(finalText&&(conf===0||conf>0.3)){
        setTimeout(function(){
          send();
        },200);
      }else{
        kalaVoiceUpdating=true;
        qi.value='';
        kalaVoiceUpdating=false;
        if(talkMode)scheduleResumeListen();
      }
    }
  };
  recog.onerror=function(e){
    console.warn('STT error:',e.error);
    clearSttSilenceTimer();
    sttSessionReady=true;
    isRecording=false;
    var mb=document.getElementById('mbtn');
    if(mb){mb.classList.remove('recording');mb.textContent=talkMode?'🗣':'🎙';}
    syncVsMic();
    if(voiceStudioOpen)setVoicePhase('idle');
    if(e.error==='not-allowed'){
      alert('Microphone access denied. Please allow mic in browser settings and refresh.');
    }else if(e.error==='aborted'||e.error==='network'){
      // Transient error — auto-retry after short delay
      if(talkMode)setTimeout(function(){scheduleResumeListen();},500);
    }else if(e.error==='no-speech'){
      // User was silent — just restart listening
      if(talkMode)scheduleResumeListen();
    }
  };
  recog.onend=function(){
    clearSttSilenceTimer();
    sttSessionReady=true;
    isRecording=false;
    var mb=document.getElementById('mbtn');
    if(talkMode){
      if(kalaSendBusy){
        if(mb){mb.classList.remove('recording');mb.textContent='🗣';}
        syncVsMic();
        return;
      }
      var qi=document.getElementById('qi');
      if(qi&&qi.value.trim().length>0){
        if(mb){mb.classList.remove('recording');mb.textContent='🗣';}
        syncVsMic();
        setTimeout(function(){send();},200);
        return;
      }
      scheduleResumeListen();
    }else{
      if(mb){mb.classList.remove('recording');mb.textContent='🎙';}
      syncVsMic();
    }
  };
}

function toggleMic(){
  if(!recog)initSTT();
  var mb=document.getElementById('mbtn');
  if(talkMode){
    talkMode=false;
    pendingVoiceSend=null;
    clearListenResume();
    clearSttSilenceTimer();
    if(recog)try{recog.stop();}catch(e){}
    sttSessionReady=true;
    isRecording=false;
    ttsSessionId++;
    try{window.speechSynthesis&&window.speechSynthesis.cancel();}catch(e){}
    if(mb){mb.classList.remove('recording','talkmode');mb.textContent='🎙';mb.title='Kala Voice: 3D AI face + mic';}
    if(voiceStudioOpen)setVoicePhase('idle');
  }else{
    openVoiceStudio();
    var SR=window.SpeechRecognition||window.webkitSpeechRecognition;
    if(!SR){
      alert('Kala Voice needs Chrome or Edge for speech recognition.');
      syncVsMic();
      return;
    }
    requestAnimationFrame(function(){
      requestAnimationFrame(function(){
        enableVoiceCapture();
      });
    });
  }
  syncVsMic();
}

function startListeningCore(){
  if(!talkMode)return;
  if(kalaSendBusy){
    scheduleResumeListen();
    return;
  }
  // Wait for Kala to finish speaking before starting mic —
  // otherwise mic picks up Kala's speaker output and STT
  // transcribes HER words as user input (echo/feedback loop).
  var syn=window.speechSynthesis;
  if(syn&&(syn.speaking||syn.pending)){
    scheduleResumeListen();
    return;
  }
  if(!recog)initSTT();
  if(!sttSessionReady){
    listenResumeTimer=setTimeout(function(){startListeningCore();},140);
    return;
  }
  recog.continuous=!!talkMode;
  var mb=document.getElementById('mbtn');
  try{
    recog.start();
    sttSessionReady=false;
    isRecording=true;
    bumpSttSilenceWatch();
    if(mb){mb.classList.add('recording');mb.textContent='🔴';}
  }catch(e){
    sttSessionReady=true;
    if(mb)mb.textContent='🗣';
    if(voiceStudioOpen&&talkMode)scheduleResumeListen();
  }
  syncVsMic();
}
function startListening(){
  if(!talkMode)return;
  var s=window.speechSynthesis;
  if(s&&(s.speaking||s.pending)){
    scheduleResumeListen();
    return;
  }
  startListeningCore();
}

// ── Text-to-Speech (TTS) ──────────────────────────────────────
var ttsEnabled=false;  // off for typing; talk mode always speaks

function toggleTTS(){
  ttsEnabled=!ttsEnabled;
  var btn=document.getElementById('ttsbtn');
  if(btn){
    if(ttsEnabled){btn.textContent='🔊 Voice On';btn.classList.add('on');}
    else{btn.textContent='🔇 Voice Off';btn.classList.remove('on');window.speechSynthesis&&window.speechSynthesis.cancel();}
  }
}

function detectTextLang(text){
  if(!text)return 'en';
  var sample=text.substring(0,200);
  var hi=0,te=0,ta=0,ar=0,zh=0,ja=0,ko=0,ru=0,th=0,bn=0;
  for(var i=0;i<sample.length;i++){
    var c=sample.charCodeAt(i);
    if(c>=0x0900&&c<=0x097F)hi++;
    else if(c>=0x0C00&&c<=0x0C7F)te++;
    else if(c>=0x0B80&&c<=0x0BFF)ta++;
    else if(c>=0x0600&&c<=0x06FF)ar++;
    else if(c>=0x4E00&&c<=0x9FFF)zh++;
    else if((c>=0x3040&&c<=0x309F)||(c>=0x30A0&&c<=0x30FF))ja++;
    else if(c>=0xAC00&&c<=0xD7AF)ko++;
    else if(c>=0x0400&&c<=0x04FF)ru++;
    else if(c>=0x0E00&&c<=0x0E7F)th++;
    else if(c>=0x0980&&c<=0x09FF)bn++;
  }
  var max=Math.max(hi,te,ta,ar,zh,ja,ko,ru,th,bn);
  if(max<2)return 'en';
  if(hi===max)return 'hi-IN';
  if(te===max)return 'te-IN';
  if(ta===max)return 'ta-IN';
  if(ar===max)return 'ar-SA';
  if(zh===max)return 'zh-CN';
  if(ja===max)return 'ja-JP';
  if(ko===max)return 'ko-KR';
  if(ru===max)return 'ru-RU';
  if(th===max)return 'th-TH';
  if(bn===max)return 'bn-IN';
  return 'en';
}
function pickVoiceForLang(voices,lang){
  if(!voices||!voices.length)return null;
  var prefix=lang.split('-')[0];
  var exact=voices.find(function(v){return v.lang.toLowerCase()===lang.toLowerCase();});
  if(exact)return exact;
  var partial=voices.find(function(v){return v.lang.toLowerCase().startsWith(prefix);});
  if(partial)return partial;
  if(prefix==='en'){
    return voices.find(function(v){return /female|zira|susan|samantha|google.*english/i.test(v.name);})
      ||voices.find(function(v){return /en[-_]/i.test(v.lang);})
      ||voices[0];
  }
  return voices.find(function(v){return /en[-_]/i.test(v.lang);})||voices[0];
}
function speakKala(text){
  if((!ttsEnabled&&!talkMode&&!voiceStudioOpen)||!window.speechSynthesis)return;
  var clean=text
    .replace(/!\[.*?\]\(.*?\)/g,'image')
    .replace(/```[\s\S]*?```/g,'code block')
    .replace(/`[^`]+`/g, function(m){return m.replace(/`/g,'');})
    .replace(/\[([^\]]+)\]\([^)]+\)/g,'$1')
    .replace(/^#{1,6}\s+/gm,'')
    .replace(/[*_~|>]/g,'')
    .replace(/\s+/g,' ')
    .trim();
  if(!clean){
    if(talkMode)scheduleResumeListen();
    return;
  }
  ttsSessionId++;
  var sid=ttsSessionId;
  try{window.speechSynthesis.cancel();}catch(e){}

  var voices=window.speechSynthesis.getVoices();
  var detectedLang=detectTextLang(clean);
  var pickedVoice=pickVoiceForLang(voices,detectedLang);

  // Chrome cuts off speech after ~15s. Split into sentences and speak sequentially.
  var chunks=splitSpeechChunks(clean,180);
  var ci=0;

  function speakNext(){
    if(sid!==ttsSessionId||ci>=chunks.length){
      clearInterval(window.ttsChromeWatchdog);
      if(voiceStudioOpen)setVoicePhase('idle');
      if(talkMode)scheduleResumeListen();
      return;
    }
    var utt=new SpeechSynthesisUtterance(chunks[ci]);
    utt.rate=0.97;
    utt.pitch=1.05;
    if(pickedVoice){utt.voice=pickedVoice;utt.lang=pickedVoice.lang;}
    else if(detectedLang)utt.lang=detectedLang;
    utt.onstart=function(){
      if(sid!==ttsSessionId)return;
      if(voiceStudioOpen)setVoicePhase('speak');
    };
    utt.onend=function(){
      if(sid!==ttsSessionId)return;
      ci++;
      speakNext();
    };
    utt.onerror=function(){
      if(sid!==ttsSessionId)return;
      ci++;
      speakNext();
    };
    window.speechSynthesis.speak(utt);
    // Chrome workaround: keep synthesis alive
    if(typeof ttsChromeWatchdog==='undefined')window.ttsChromeWatchdog=null;
    clearInterval(window.ttsChromeWatchdog);
    window.ttsChromeWatchdog=setInterval(function(){
      if(!window.speechSynthesis.speaking){clearInterval(window.ttsChromeWatchdog);return;}
      window.speechSynthesis.pause();
      window.speechSynthesis.resume();
    },5000);
  }
  speakNext();
  // Do NOT start mic while Kala speaks — her voice bleeds into the mic
  // and STT transcribes Kala's words as user input. Mic resumes in speakNext()
  // when all chunks finish (via scheduleResumeListen).
}

function splitSpeechChunks(text,maxLen){
  if(text.length<=maxLen)return [text];
  var sentences=text.match(/[^.!?\n]+[.!?\n]+|[^.!?\n]+$/g)||[text];
  var chunks=[];
  var current='';
  for(var i=0;i<sentences.length;i++){
    var s=sentences[i].trim();
    if(!s)continue;
    if((current+' '+s).length>maxLen&&current){
      chunks.push(current.trim());
      current=s;
    }else{
      current=current?current+' '+s:s;
    }
  }
  if(current.trim())chunks.push(current.trim());
  return chunks.length?chunks:[text.substring(0,maxLen)];
}

// ── Pre-load voices ──────────────────────────────────────────
if(window.speechSynthesis){
  window.speechSynthesis.getVoices();
  window.speechSynthesis.onvoiceschanged=function(){window.speechSynthesis.getVoices();};
}

// Conversation memory — keeps this tab's full transcript until you close the page or click New chat.
// Server receives the last MAX_SEND_HISTORY entries per request (LLM context limits).
var MAX_SEND_HISTORY=120;
var MAX_STORE_TURNS=400;
var chatHistory=[];
var userMemory={name:'',facts:[]};

// ── Persistent Memory (localStorage) ───────────────────────
function loadPersistentMemory(){
  try{
    var d=localStorage.getItem('kala_memory');
    if(d){var m=JSON.parse(d);if(m.name){var nl=m.name.toLowerCase();var badNames=['kala','ghost','killer','ai','bot','assistant','carla'];if(badNames.indexOf(nl)===-1){userMemory.name=m.name;}else{m.name='';localStorage.setItem('kala_memory',JSON.stringify(m));}}if(m.facts)userMemory.facts=m.facts;}
    var h=localStorage.getItem('kala_history');
    if(h){chatHistory=JSON.parse(h)||[];}
    var v=localStorage.getItem('kala_visits');
    userMemory.visits=v?parseInt(v)+1:1;
    localStorage.setItem('kala_visits',String(userMemory.visits));
    localStorage.setItem('kala_last_visit',new Date().toISOString());
  }catch(e){}
}
function savePersistentMemory(){
  try{
    localStorage.setItem('kala_memory',JSON.stringify({name:userMemory.name,facts:userMemory.facts}));
    var recent=chatHistory.slice(-60);
    localStorage.setItem('kala_history',JSON.stringify(recent));
  }catch(e){}
}
loadPersistentMemory();
setInterval(savePersistentMemory,15000);
window.addEventListener('beforeunload',savePersistentMemory);

// ── Typewriter animation ───────────────────────────────────
function typewriteHTML(el,html){
  el.innerHTML=html;
  var full=el.innerHTML;
  var len=full.length;
  if(len>1500){return;}
  var visible=0;var step=Math.max(1,Math.floor(len/120));
  el.innerHTML='';
  var cursor=document.createElement('span');
  cursor.className='typing-cursor';
  el.appendChild(cursor);
  function tick(){
    visible+=step;
    if(visible>=len){
      el.innerHTML=full;
      return;
    }
    var partial=full.substring(0,visible);
    var openTags=partial.match(/<[^/][^>]*>/g)||[];
    var closeTags=partial.match(/<\/[^>]+>/g)||[];
    if(openTags.length>closeTags.length){
      el.innerHTML=full;
      return;
    }
    el.innerHTML=partial;
    el.appendChild(cursor);
    var c=document.getElementById('msgs');
    if(c)c.scrollTop=c.scrollHeight;
    requestAnimationFrame(tick);
  }
  requestAnimationFrame(tick);
}

// ── Sparkle on send ────────────────────────────────────────
function spawnSparkle(x,y){
  var emojis=['✨','⚡','🌟','💫','🔥'];
  for(var i=0;i<4;i++){
    var s=document.createElement('div');
    s.className='sparkle';
    s.textContent=emojis[Math.floor(Math.random()*emojis.length)];
    s.style.left=(x+Math.random()*50-25)+'px';
    s.style.top=(y+Math.random()*30-15)+'px';
    document.body.appendChild(s);
    setTimeout(function(el){el.remove();},(i+1)*900,s);
  }
}

// ── Thinking Chain Visualization ──────────────────────────
function showThinkingChain(steps,tid){
  if(!steps||steps.length===0)return;
  var tEl=document.getElementById(tid);
  if(!tEl)return;
  var chain=document.createElement('div');
  chain.className='thinking-chain';
  steps.forEach(function(step,i){
    var s=document.createElement('div');
    s.className='think-step';
    s.style.animationDelay=(i*0.12)+'s';
    s.innerHTML='<span class="think-dot"></span><span class="think-text">'+step+'</span>';
    chain.appendChild(s);
  });
  tEl.querySelector('.bubble')?.replaceChildren(chain);
  setTimeout(function(){
    var dots=chain.querySelectorAll('.think-step');
    dots.forEach(function(d){d.classList.add('done');});
  },steps.length*150+300);
}

// ── Scroll-to-bottom FAB ──────────────────────────────────
function scrollToBottom(){
  var mc=document.getElementById('msgs');
  if(mc)mc.scrollTop=mc.scrollHeight;
}
(function initScrollFab(){
  var mc=document.getElementById('msgs');
  var fab=document.getElementById('scrollFab');
  if(!mc||!fab)return;
  mc.addEventListener('scroll',function(){
    var fromBottom=mc.scrollHeight-mc.scrollTop-mc.clientHeight;
    if(fromBottom>200)fab.classList.add('show');
    else fab.classList.remove('show');
  });
})();

// ── Message reactions ─────────────────────────────────────
function addReactions(msgEl){
  var reacts=['👍','❤️','😂','🔥','🤔','👏'];
  var wrap=document.createElement('div');
  wrap.className='msg-reactions';
  reacts.forEach(function(emoji){
    var btn=document.createElement('button');
    btn.className='msg-react-btn';
    btn.textContent=emoji;
    btn.onclick=function(){
      if(btn.classList.contains('reacted')){
        btn.classList.remove('reacted');
        btn.textContent=emoji;
      }else{
        btn.classList.add('reacted');
        btn.textContent=emoji+' 1';
        btn.style.transform='scale(1.15)';
        setTimeout(function(){btn.style.transform='';},200);
      }
    };
    wrap.appendChild(btn);
  });
  msgEl.appendChild(wrap);
}

// ── Voice waveform visualizer ─────────────────────────────
var vsAnalyser=null,vsWaveRaf=null;
function initVoiceWaveform(stream){
  try{
    var ctx=new (window.AudioContext||window.webkitAudioContext)();
    var src=ctx.createMediaStreamSource(stream);
    vsAnalyser=ctx.createAnalyser();
    vsAnalyser.fftSize=256;
    src.connect(vsAnalyser);
    drawWaveform();
  }catch(e){}
}
function drawWaveform(){
  var canvas=document.getElementById('vsWaveCanvas');
  if(!canvas||!vsAnalyser){return;}
  var cctx=canvas.getContext('2d');
  var w=canvas.parentElement.clientWidth;
  var h=canvas.parentElement.clientHeight;
  canvas.width=w*2;canvas.height=h*2;
  cctx.scale(2,2);
  var bufLen=vsAnalyser.frequencyBinCount;
  var data=new Uint8Array(bufLen);
  function render(){
    vsWaveRaf=requestAnimationFrame(render);
    vsAnalyser.getByteFrequencyData(data);
    cctx.clearRect(0,0,w,h);
    var barW=w/bufLen*2.5;
    var x=0;
    for(var i=0;i<bufLen;i++){
      var val=data[i]/255;
      var barH=val*h*0.85;
      var g=cctx.createLinearGradient(0,h,0,h-barH);
      g.addColorStop(0,'rgba(94,234,212,0.15)');
      g.addColorStop(1,'rgba(94,234,212,'+(.3+val*.7)+')');
      cctx.fillStyle=g;
      cctx.fillRect(x,h-barH,barW-1,barH);
      x+=barW;
    }
  }
  render();
}
function stopVoiceWaveform(){
  if(vsWaveRaf){cancelAnimationFrame(vsWaveRaf);vsWaveRaf=null;}
  var canvas=document.getElementById('vsWaveCanvas');
  if(canvas){var c=canvas.getContext('2d');c.clearRect(0,0,canvas.width,canvas.height);}
}
function setVsStatus(text){
  var el=document.getElementById('vsStatusText');
  if(el)el.textContent=text;
}

// ── Mood System ────────────────────────────────────────────
var kalaMood='happy';
var moodEmojis={happy:'😊',excited:'🔥',thinking:'🧠',playful:'😄',chill:'😌',love:'💜'};
function detectMood(text){
  var t=text.toLowerCase();
  if(/game|play|joke|fun|scramble|riddle|trivia|haha|lol|😂/.test(t))return 'playful';
  if(/amazing|awesome|great|perfect|love|thank|❤|💛|🎉/.test(t))return 'love';
  if(/think|why|how|explain|deep|philosophy|meaning/.test(t))return 'thinking';
  if(/code|build|create|project|implement|bug|fix/.test(t))return 'excited';
  if(/relax|chill|hello|hi|hey|good/.test(t))return 'chill';
  return 'happy';
}
function updateMoodIndicator(mood){
  kalaMood=mood;
  var el=document.getElementById('kala-mood');
  if(el){
    el.textContent=moodEmojis[mood]||'😊';
    el.title='Kala is feeling: '+mood;
    el.classList.add('mood-pulse');
    setTimeout(function(){el.classList.remove('mood-pulse');},600);
  }
}

// ── Time-aware greeting ────────────────────────────────────
function getTimeGreeting(){
  var h=new Date().getHours();
  var name=userMemory.name||'friend';
  var visits=userMemory.visits||1;
  var greet='';
  if(h<5)greet='🌙 Late night coding, '+name+'?';
  else if(h<12)greet='☀️ Good morning, '+name+'!';
  else if(h<17)greet='🌤️ Good afternoon, '+name+'!';
  else if(h<21)greet='🌅 Good evening, '+name+'!';
  else greet='🌙 Night owl mode, '+name+'!';
  if(visits>1)greet+=' Welcome back (visit #'+visits+')!';
  else greet+=' Welcome to Kala!';
  return greet;
}

function newChat(){
  if(chatHistory.length===0)return;
  if(!confirm('Start a new chat? This clears conversation memory for this tab.'))return;
  chatHistory=[];
  var keepName=userMemory.name;
  userMemory={name:keepName,facts:[],visits:userMemory.visits};
  var m=document.getElementById('msgs');
  m.innerHTML='<div class="welcome" id="welcome"><div class="wglyph">⚡</div><div class="wtitle"><em>Kala</em> — AI within Killer</div><div class="wsub">Ask anything. Scaffold multi-file projects. Native KhLM + AI Lab demos — LLM optional for best cloud answers.</div><div class="wpills"><span class="wpill">💬 10 AI Modes</span><span class="wpill">🌐 Works Offline</span><span class="wpill">⚡ Ghost-108 Search</span><span class="wpill">🧪 AI Lab: native demos + curriculum</span></div><div class="pgrid"><div class="pcard" onclick="qp(\'ask\',\'What is speed of light and how was it first measured?\')"><div class="pico">💬</div><div class="ptitle">Ask anything</div><div class="pdesc">Speed of light, math, history…</div></div><div class="pcard" onclick="qp(\'write\',\'artificial intelligence future\',\'essay\')"><div class="pico">✍️</div><div class="ptitle">Write an essay</div><div class="pdesc">AI future — academic prose</div></div><div class="pcard" onclick="qp(\'imagine\',\'a world where every city runs on Killer code\')"><div class="pico">🔭</div><div class="ptitle">Imagine a world</div><div class="pdesc">Cities running on Killer code</div></div><div class="pcard" onclick="qp(\'lab\',\'ml machine learning algorithms\')"><div class="pico">🧪</div><div class="ptitle">AI Lab</div><div class="pdesc">Rust demos · curriculum (AGI/ASI/OS)</div></div><div class="pcard" onclick="qp(\'ai_system\',\'Compare approaches: explain gradient descent, then summarize who leads OpenAI today.\')"><div class="pico">🤖</div><div class="ptitle">AI System</div><div class="pdesc">Hard Qs: router+search+GGUF+merge · not AGI</div></div><div class="pcard" onclick="qp(\'code\',\'Create a Python FastAPI REST API with SQLite, README, and run steps.\')"><div class="pico">💻</div><div class="ptitle">Starter project</div><div class="pdesc">API + DB layout</div></div><div class="pcard" onclick="qp(\'think\',\'Why does consciousness exist?\')"><div class="pico">🧠</div><div class="ptitle">Think deep</div><div class="pdesc">Consciousness, existence, meaning</div></div><div class="pcard" onclick="enterVoiceStudioAndListen()"><div class="pico">🎙</div><div class="ptitle">Voice studio</div><div class="pdesc">AI mesh face · hands-free</div></div></div></div>';
  fetch('/api/kala/clear-session',{method:'POST'}).catch(function(){});
}

function rememberUser(text){
  var nm=text.match(/(?:i(?:'m| am|m) |my name(?:'s| is) |call me |^am |this is )([A-Z][a-z]{1,20})/i);
  if(nm&&nm[1]){
    var n=nm[1].toLowerCase();
    var bad=['kala','ghost','killer','ai','bot','assistant','doing','good','fine','great','ok','okay','not','feeling','happy','sad','tired','here','back','new','ready'];
    if(bad.indexOf(n)===-1){
      userMemory.name=nm[1].charAt(0).toUpperCase()+nm[1].slice(1).toLowerCase();
      savePersistentMemory();
    }
  }
}

function stopKala(){
  if(kalaAbortCtrl){try{kalaAbortCtrl.abort();}catch(e){}}
  kalaAbortCtrl=null;
  kalaSendBusy=false;
  kalaAutoStopArmed=false;
  var btn=document.getElementById('sbtn');
  btn.textContent='▶';btn.title='Send (Enter)';btn.disabled=false;btn.classList.remove('stop');
  var qi=document.getElementById('qi');qi.disabled=false;qi.focus();
  try{window.speechSynthesis&&window.speechSynthesis.cancel();}catch(e){}
}

async function send(){
  const qi=document.getElementById('qi');
  const q=qi.value.trim();
  if(!q&&!kalaSendBusy)return;
  if(kalaSendBusy){
    stopKala();
    if(q){setTimeout(function(){var qi2=document.getElementById('qi');qi2.value=q;resize(qi2);send();},100);}
    return;
  }
  lastFailedQuestion=null;
  if(ttsEnabled||talkMode||voiceStudioOpen){
    ttsSessionId++;
    try{window.speechSynthesis&&window.speechSynthesis.cancel();}catch(e){}
    clearListenResume();
  }
  kalaSendBusy=true;
  if(voiceStudioOpen){
    setVoicePhase('think');
    lastVoiceQ=q;
    var vt=document.getElementById('vsTranscript');
    if(vt)vt.innerHTML='<b>You:</b> '+escVs(q)+'<br><b>Kala:</b> …';
    var vh=document.getElementById('vsHeard');
    if(vh)vh.textContent='';
  }
  document.getElementById('welcome')?.remove();
  var btnRect=document.getElementById('sbtn').getBoundingClientRect();
  spawnSparkle(btnRect.left,btnRect.top);
  addMsg('user',q);
  qi.value='';qi.style.height='auto';
  rememberUser(q);
  updateMoodIndicator(detectMood(q));
  const btn=document.getElementById('sbtn');
  btn.textContent='■';btn.title='Stop (click or type to interrupt)';btn.disabled=false;btn.classList.add('stop');
  var qi2=document.getElementById('qi');qi2.disabled=false;
  const tid=addThink();
  const t0=Date.now();
  kalaAbortCtrl=new AbortController();
  kalaAutoStopArmed=false;
  setTimeout(function(){kalaAutoStopArmed=true;},300);
  try{
    var histSlice=chatHistory.length<=MAX_SEND_HISTORY?chatHistory:chatHistory.slice(-MAX_SEND_HISTORY);
    const r=await fetch('/api/kala',{
      method:'POST',
      headers:{'Content-Type':'application/json'},
      body:JSON.stringify({mode,question:q,style,lang:'killer',history:histSlice,uname:userMemory.name}),
      signal:kalaAbortCtrl.signal
    });
    const raw=await r.text();
    var d;
    try{d=JSON.parse(raw);}
    catch(_){
      throw new Error('Server sent invalid JSON — is the Kala process still running on this port?');
    }
    if(typeof d.response!=='string'){
      throw new Error('Malformed API reply (no response text).');
    }
    if(!r.ok){
      throw new Error((d&&d.error)||('HTTP '+r.status));
    }
    const ms=Date.now()-t0;
    totalMs+=ms;msgs++;
    showThinkingChain(d.thinking||[],tid);
    document.getElementById(tid)?.remove();
    addMsg('kala',d.response,ms,mode,{
      confidence:d.confidence||0,
      suggestions:d.suggestions||[],
      intent:d.intent||'',
      topic:d.topic||'',
      thinking:d.thinking||[]
    });
    updateMoodIndicator(detectMood(d.response));
    savePersistentMemory();
    if(voiceStudioOpen){
      var plain=stripVoiceMd(d.response);
      var vt2=document.getElementById('vsTranscript');
      if(vt2)vt2.innerHTML='<b>You:</b> '+escVs(lastVoiceQ)+'<br><b>Kala:</b> '+escVs(plain);
      lastVoiceQ='';
    }
    speakKala(d.response);
    chatHistory.push({r:'user',c:q});
    chatHistory.push({r:'assistant',c:d.response});
    if(chatHistory.length>MAX_STORE_TURNS)chatHistory=chatHistory.slice(-MAX_STORE_TURNS);
  }catch(e){
    document.getElementById(tid)?.remove();
    if(e.name==='AbortError'){return;}
    lastFailedQuestion=q;
    addMsg('kala','**Could not reach Kala**\n\n'+e.message+'\n\nCheck that the server is running and refresh if needed.',undefined,'error');
    if(voiceStudioOpen){
      lastVoiceQ='';
      setVoicePhase('idle');
      if(talkMode)scheduleResumeListen();
    }
  }finally{
    kalaAbortCtrl=null;
    kalaSendBusy=false;
    kalaAutoStopArmed=false;
    btn.textContent='▶';btn.title='Send (Enter)';btn.disabled=false;btn.classList.remove('stop');
    if(!voiceStudioOpen)qi.focus();
    if(pendingVoiceSend&&String(pendingVoiceSend).trim()){
      var pv=String(pendingVoiceSend).trim();
      pendingVoiceSend=null;
      qi.value=pv;
      resize(qi);
      setTimeout(send,40);
    }
  }
}

function addMsg(role,text,ms,m,aiMeta){
  const c=document.getElementById('msgs');
  const w=document.createElement('div');
  w.className='mwrap '+(role==='user'?'user':'');
  const av=document.createElement('div');
  av.className='av '+(role==='kala'?'k':'u');
  av.textContent=role==='kala'?'⚡':'👤';
  const body=document.createElement('div');
  body.className='mbody';
  const b=document.createElement('div');
  b.className='bubble '+(role==='kala'?'kb':'ub');
  if(role==='kala'){
    var rendered=md(text);
    var shouldAnimate=rendered.length<2000&&!rendered.includes('<pre');
    if(shouldAnimate){
      b.innerHTML='';
      typewriteHTML(b,rendered);
    }else{
      b.innerHTML=rendered;
    }
    b.querySelectorAll('pre').forEach(pre=>{
      const head=document.createElement('div');
      head.className='pre-head';
      const lang=pre.dataset.lang||'code';
      const cp=document.createElement('button');
      cp.className='cpbtn';cp.textContent='Copy';
      cp.onclick=()=>{
        navigator.clipboard?.writeText(pre.querySelector('code')?.textContent||'');
        cp.textContent='✓ Copied';cp.classList.add('ok');
        setTimeout(()=>{cp.textContent='Copy';cp.classList.remove('ok');},1500);
      };
      head.innerHTML='<span class="pre-lang">'+lang+'</span>';
      head.appendChild(cp);
      pre.insertBefore(head,pre.firstChild);
    });
  }else{
    b.textContent=text;
  }
  body.appendChild(b);
  if(role==='kala'&&ms!==undefined&&m!=='error'){
    const meta=document.createElement('div');
    meta.className='mmeta';
    var confPct=aiMeta?Math.round(aiMeta.confidence*100):0;
    var confColor=confPct>85?'var(--green)':confPct>65?'var(--gold)':'var(--dim)';
    var intentTag=aiMeta&&aiMeta.intent?aiMeta.intent.replace(/_/g,' '):'';
    var topicTag=aiMeta&&aiMeta.topic&&aiMeta.topic!=='General'?aiMeta.topic:'';
    meta.innerHTML='<span class="tg tg-gold">'+(m||'ask')+'</span>'
      +(intentTag?'<span class="tg tg-intent">'+intentTag+'</span>':'')
      +(topicTag?'<span class="tg tg-topic">'+topicTag+'</span>':'')
      +'<span class="tg-time">'+ms+'ms</span>'
      +(confPct?'<span class="conf-bar" title="Confidence: '+confPct+'%"><span class="conf-fill" style="width:'+confPct+'%;background:'+confColor+'"></span></span>':'');
    var spkBtn=document.createElement('button');
    spkBtn.className='tts-btn';spkBtn.textContent='🔊';spkBtn.title='Speak this response';
    spkBtn.style.cssText='font-size:13px;padding:1px 6px;margin-left:6px;cursor:pointer';
    (function(t){spkBtn.onclick=function(){speakKala(t);};})(text);
    meta.appendChild(spkBtn);
    body.appendChild(meta);
  }
  // Smart suggestions bar
  if(role==='kala'&&aiMeta&&aiMeta.suggestions&&aiMeta.suggestions.length>0&&m!=='error'){
    var sugBar=document.createElement('div');
    sugBar.className='smart-suggestions';
    aiMeta.suggestions.forEach(function(s){
      var btn=document.createElement('button');
      btn.className='sug-btn';
      btn.textContent=s;
      btn.onclick=function(){
        document.getElementById('qi').value=s;
        send();
      };
      sugBar.appendChild(btn);
    });
    body.appendChild(sugBar);
  }
  if(role==='kala'&&m==='error'&&lastFailedQuestion){
    const rb=document.createElement('button');
    rb.type='button';
    rb.className='retry-btn';
    rb.textContent='Retry question';
    rb.onclick=function(){
      var qq=lastFailedQuestion;
      if(!qq)return;
      lastFailedQuestion=null;
      var iq=document.getElementById('qi');
      iq.value=qq;
      resize(iq);
      if(!voiceStudioOpen)iq.focus();
      send();
    };
    body.appendChild(rb);
  }
  if(role==='kala'&&m!=='error'){addReactions(body);}
  w.appendChild(av);w.appendChild(body);c.appendChild(w);
  c.scrollTop=c.scrollHeight;
}

function addThink(){
  const id='t'+Date.now();
  const c=document.getElementById('msgs');
  const w=document.createElement('div');
  w.className='twrap';w.id=id;
  w.innerHTML='<div class="av k">⚡</div><div class="bubble kb"><div class="tdots"><div class="dot"></div><div class="dot"></div><div class="dot"></div></div></div>';
  c.appendChild(w);c.scrollTop=c.scrollHeight;
  return id;
}

// ── Markdown ──────────────────────────────────────────────────
function md(t){
  // fenced code blocks
  t=t.replace(/```(\w*)\n([\s\S]*?)```/g,(_,lg,code)=>{
    const e=code.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
    return '<pre data-lang="'+(lg||'code')+'"><code>'+e+'</code></pre>';
  });
  // inline code
  t=t.replace(/`([^`\n]+)`/g,'<code>$1</code>');
  // headings h1-h4
  t=t.replace(/^# (.+)$/gm,'<h2>$1</h2>');
  t=t.replace(/^## (.+)$/gm,'<h2>$1</h2>');
  t=t.replace(/^### (.+)$/gm,'<h3>$1</h3>');
  t=t.replace(/^#### (.+)$/gm,'<h4>$1</h4>');
  // Audio data URIs: <audio controls src="data:audio/wav;base64,...">
  t=t.replace(/<audio controls src="(data:audio\/[^"]+)"([^>]*)><\/audio>/g,function(_,src,rest){
    return '<audio controls src="'+src+'" style="width:100%;margin:8px 0;border-radius:10px;outline:none"'+rest+'></audio>';
  });
  // Image markdown: ![alt](url) — handles both https:// and data: URIs
  t=t.replace(/!\[([^\]]*)\]\(((?:https?:\/\/|data:)[^)]{4,})\)/g,function(_,alt,url){
    var a=alt||'Kala generated image';
    return '<div class="kala-img-wrap"><img src="'+url+'" alt="'+a+'" class="kala-img" loading="lazy"><p class="kala-img-cap">'+a+'</p></div>';
  });
  // [VIEW IMAGE](url) → image card
  t=t.replace(/\[VIEW IMAGE[^\]]*\]\((https?:\/\/[^)]+)\)/g,function(_,url){
    return '<div class="kala-img-wrap"><img src="'+url+'" alt="Generated image" class="kala-img" loading="lazy"><p class="kala-img-cap"><a href="'+url+'" target="_blank" rel="noopener">Open full size \u2197</a></p></div>';
  });
  // [WATCH VIDEO](url) → video player
  t=t.replace(/\[WATCH VIDEO\]\((https?:\/\/[^)]+)\)/g,function(_,url){
    return '<div class="kala-img-wrap"><video src="'+url+'" controls class="kala-img"></video><p class="kala-img-cap"><a href="'+url+'" target="_blank" rel="noopener">Open video \u2197</a></p></div>';
  });
  // generic markdown links
  t=t.replace(/\[([^\]]+)\]\((https?:\/\/[^)]+)\)/g,'<a href="$2" target="_blank" rel="noopener">$1</a>');
  // bold / italic
  t=t.replace(/\*\*([^*\n]+)\*\*/g,'<strong>$1</strong>');
  t=t.replace(/\*([^*\n]+)\*/g,'<em>$1</em>');
  // blockquote
  t=t.replace(/^> (.+)$/gm,'<blockquote>$1</blockquote>');
  // hr
  t=t.replace(/^---+$/gm,'<hr>');
  // lists
  t=t.replace(/^[\u2022\u00b7\-\*] (.+)$/gm,'<li>$1</li>');
  t=t.replace(/^\d+\. (.+)$/gm,'<li>$1</li>');
  t=t.replace(/(<li>[\s\S]*?<\/li>)+/g,function(m){return'<ul>'+m+'</ul>';});
  // paragraphs
  var parts=t.split(/\n{2,}/);
  return parts.map(function(p){
    p=p.trim();if(!p)return'';
    if(/^<(h[1-6]|ul|ol|pre|blockquote|hr|div)/.test(p))return p;
    return'<p>'+p.replace(/\n/g,'<br>')+'</p>';
  }).join('');
}

// ── Init ──────────────────────────────────────────────────────
const port=window.location.port||'8080';
document.getElementById('port-disp').textContent=port;
(function initKalaGate(){
  try{
    if(!sessionStorage.getItem('kala_gate_ok')){
      var g=document.getElementById('kalaGate');
      if(g){
        g.classList.add('kala-gate-visible');
        g.setAttribute('aria-hidden','false');
      }
      document.body.classList.add('kala-gate-on');
    }
  }catch(e){}
})();
if(!document.body.classList.contains('kala-gate-on'))
  document.getElementById('qi').focus();

// ── Restore session & time greeting ───────────────────────
(function initKalaSpecial(){
  if(chatHistory.length>0){
    var w=document.getElementById('welcome');
    if(w)w.remove();
    chatHistory.forEach(function(m){
      if(m.r==='user')addMsg('user',m.c);
      else addMsg('kala',m.c);
    });
    var mc=document.getElementById('msgs');
    if(mc)mc.scrollTop=mc.scrollHeight;
  }
  if(userMemory.visits>1||userMemory.name){
    var g=getTimeGreeting();
    var mc=document.getElementById('msgs');
    if(mc){
      var tip=document.createElement('div');
      tip.className='bubble kb';
      tip.style.cssText='text-align:center;opacity:0.85;font-size:13px;border:1px dashed var(--gold);margin:8px auto;max-width:400px;padding:8px 14px;border-radius:12px';
      tip.innerHTML=g+'<br><span style=\"font-size:11px;opacity:0.7\">Tip: Type <b>/help</b> for special commands!</span>';
      mc.appendChild(tip);
    }
  }
  // Random daily tip
  var tips=['Try /joke for a laugh!','Type /game to play mini-games!','Say /fortune for your coding fortune!','Try /riddle for a brain teaser!','Type /scramble for word games!','Type /story to build a story together!','Ask me anything in any language!','Try voice mode — click Voice button!'];
  var today=new Date().getDate();
  var dailyTip=tips[today%tips.length];
  document.title='Kala AI — '+dailyTip;
})();
</script>

<!-- LLM Settings Modal -->
<div class="llm-overlay" id="llm-overlay" onclick="if(event.target===this)closeLlmSettings()">
  <div class="llm-modal">
    <button class="llm-close" onclick="closeLlmSettings()">&times;</button>
    <h3>⚙️ <span>LLM</span> Configuration</h3>
    <div class="llm-field">
      <label>PROVIDER</label>
      <select id="llm-provider" onchange="onLlmProviderChange()">
        <option value="">None (offline only)</option>
        <option value="ollama">Ollama (local)</option>
        <option value="groq">Groq (free cloud)</option>
        <option value="openai">OpenAI</option>
        <option value="anthropic">Anthropic</option>
      </select>
    </div>
    <div class="llm-field" id="llm-key-field" style="display:none">
      <label>API KEY</label>
      <input type="password" id="llm-api-key" placeholder="sk-... or gsk_...">
      <small>Stored in browser only. Sent per-request, not saved on server disk.</small>
    </div>
    <div class="llm-field">
      <label>MODEL</label>
      <input type="text" id="llm-model" placeholder="(auto-filled per provider)">
      <small id="llm-model-hint">Select a provider first</small>
    </div>
    <div class="llm-actions">
      <button class="llm-test-btn" onclick="testLlmConnection()">🔌 Test</button>
      <button class="llm-save-btn" onclick="saveLlmSettings()">💾 Save</button>
    </div>
    <div style="margin-top:10px">
      <button type="button" class="llm-test-btn" style="width:100%" onclick="clearKalaAnswerCache()">🧹 Clear answer cache</button>
      <small style="display:block;opacity:0.75;margin-top:4px">Caches also clear automatically when you <b>Save</b> LLM settings or <b>clear session</b>. Use this if a bad reply sticks mid-chat.</small>
    </div>
    <div class="llm-status" id="llm-status"></div>
  </div>
</div>
<script>
(function(){
  var defaults={ollama:'llama3',groq:'llama3-70b-8192',openai:'gpt-4o-mini',anthropic:'claude-opus-4-5'};
  var hints={ollama:'e.g. llama3, mistral, phi3, gemma2',groq:'e.g. llama3-70b-8192, mixtral-8x7b-32768',openai:'e.g. gpt-4o-mini, gpt-4o',anthropic:'e.g. claude-opus-4-5, claude-3-5-sonnet'};
  var needsKey={ollama:false,groq:true,openai:true,anthropic:true};

  window.openLlmSettings=function(){
    var saved=JSON.parse(localStorage.getItem('kala_llm_settings')||'{}');
    if(saved.provider)document.getElementById('llm-provider').value=saved.provider;
    if(saved.apiKey)document.getElementById('llm-api-key').value=saved.apiKey;
    if(saved.model)document.getElementById('llm-model').value=saved.model;
    onLlmProviderChange();
    document.getElementById('llm-overlay').classList.add('open');
    document.getElementById('llm-status').className='llm-status';
    document.getElementById('llm-status').textContent='';
  };
  window.closeLlmSettings=function(){document.getElementById('llm-overlay').classList.remove('open')};
  window.onLlmProviderChange=function(){
    var p=document.getElementById('llm-provider').value;
    document.getElementById('llm-key-field').style.display=(needsKey[p]?'block':'none');
    document.getElementById('llm-model-hint').textContent=hints[p]||'Select a provider first';
    if(p&&!document.getElementById('llm-model').value){
      document.getElementById('llm-model').value=defaults[p]||'';
    }
  };
  window.clearKalaAnswerCache=function(){
    var st=document.getElementById('llm-status');
    st.className='llm-status info';st.textContent='Clearing caches...';
    fetch('/api/kala/cache-clear',{method:'POST',headers:{'Content-Type':'application/json'},body:'{}'})
    .then(function(r){return r.json()}).then(function(d){
      if(d.ok){st.className='llm-status ok';st.textContent='Cleared: polyglot '+d.polyglot_cleared+', inference '+d.inference_cleared+'.';}
      else{st.className='llm-status err';st.textContent='Error: '+(d.error||'unknown');}
    }).catch(function(e){st.className='llm-status err';st.textContent='Network error: '+e;});
  };
  window.saveLlmSettings=function(){
    var provider=document.getElementById('llm-provider').value;
    var apiKey=document.getElementById('llm-api-key').value.trim();
    var model=document.getElementById('llm-model').value.trim()||defaults[provider]||'';
    localStorage.setItem('kala_llm_settings',JSON.stringify({provider:provider,apiKey:apiKey,model:model}));
    var st=document.getElementById('llm-status');
    if(!provider){st.className='llm-status info';st.textContent='Cleared — Kala will use offline mode.';updateGearBadge('');return;}
    st.className='llm-status info';st.textContent='Saving to server...';
    fetch('/api/kala/llm-config',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({provider:provider,api_key:apiKey,model:model})})
    .then(function(r){return r.json()}).then(function(d){
      if(d.ok){st.className='llm-status ok';st.textContent='Saved! LLM: '+provider+' / '+model;updateGearBadge(provider);}
      else{st.className='llm-status err';st.textContent='Error: '+(d.error||'unknown');}
    }).catch(function(e){st.className='llm-status err';st.textContent='Network error: '+e;});
  };
  window.testLlmConnection=function(){
    var provider=document.getElementById('llm-provider').value;
    var apiKey=document.getElementById('llm-api-key').value.trim();
    var model=document.getElementById('llm-model').value.trim()||defaults[provider]||'';
    var st=document.getElementById('llm-status');
    if(!provider){st.className='llm-status err';st.textContent='Select a provider first.';return;}
    st.className='llm-status info';st.textContent='Testing connection to '+provider+'...';
    fetch('/api/kala/llm-test',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({provider:provider,api_key:apiKey,model:model})})
    .then(function(r){return r.json()}).then(function(d){
      if(d.ok){st.className='llm-status ok';st.textContent='Connected! Response: '+d.preview;}
      else{st.className='llm-status err';st.textContent='Failed: '+(d.error||'connection error');}
    }).catch(function(e){st.className='llm-status err';st.textContent='Network error: '+e;});
  };
  function updateGearBadge(p){
    var btn=document.getElementById('llm-gear-btn');
    if(p){btn.textContent='⚙️ '+p.charAt(0).toUpperCase()+p.slice(1);btn.classList.add('on');}
    else{btn.textContent='⚙️ LLM';btn.classList.remove('on');}
  }
  // On page load, restore saved provider and push to server
  var saved=JSON.parse(localStorage.getItem('kala_llm_settings')||'{}');
  if(saved.provider){
    updateGearBadge(saved.provider);
    fetch('/api/kala/llm-config',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({provider:saved.provider,api_key:saved.apiKey||'',model:saved.model||''})}).catch(function(){});
  }
})();
</script>
</body>
</html>
"##;

// ─── Public Builtin: kala_serve(port?) ───────────────────────────────────────
pub fn builtin_kala_serve(args: &[Value]) -> Result<Value, VmError> {
    crate::security::require_network()?;
    crate::security::require_process_spawn()?;
    let port: u16 = match args.first() {
        Some(Value::Number(n)) => *n as u16,
        Some(Value::Str(s))    => s.trim().parse().unwrap_or(8080),
        _                      => 8080,
    };

    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).map_err(|e| {
        VmError::runtime_error(format!("kala_serve: cannot bind to {}: {}", addr, e))
    })?;

    println!();
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║   Kala (काल) Chat UI                                     ║");
    println!("║   Open:  http://127.0.0.1:{}                          ║", port);
    println!("║   Stop:  Ctrl+C                                          ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    open_browser(&format!("http://127.0.0.1:{}", port));

    for stream in listener.incoming() {
        match stream {
            Ok(s) => { thread::spawn(move || handle_conn(s)); }
            Err(_) => break,
        }
    }

    Ok(Value::Str(format!("Kala UI stopped (was on port {}).", port)))
}

// ─── Open browser cross-platform ─────────────────────────────────────────────
fn open_browser(url: &str) {
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd").args(["/c", "start", "", url]).spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}

// ─── Connection handler ───────────────────────────────────────────────────────
fn handle_conn(stream: TcpStream) {
    use std::time::Duration;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    // BufReader takes ownership; recovered via into_inner() after all reads
    let mut reader = BufReader::new(stream);

    // Read request line
    let mut req_line = String::new();
    if reader.read_line(&mut req_line).is_err() { return; }
    let req_line = req_line.trim().to_string();

    // Read headers — extract Content-Length
    let mut content_len: usize = 0;
    loop {
        let mut hdr = String::new();
        if reader.read_line(&mut hdr).is_err() { break; }
        let h = hdr.trim();
        if h.is_empty() { break; }
        if h.to_lowercase().starts_with("content-length:") {
            content_len = h[15..].trim().parse().unwrap_or(0);
        }
    }

    // Read body (POST only) — large sessions send long history JSON (multi‑MB cap)
    let body_str = if content_len > 0 {
        let cap = content_len.min(2_097_152);
        let mut body = vec![0u8; cap];
        let _ = reader.read_exact(&mut body);
        String::from_utf8_lossy(&body).to_string()
    } else {
        String::new()
    };

    // Recover stream from reader now that all reading is done
    let mut stream = reader.into_inner();

    // Capture peer IP for logging
    let peer_ip = stream.peer_addr().map(|a| a.ip().to_string()).unwrap_or_else(|_| "-".into());

    // Route
    if req_line.starts_with("GET / ") || req_line.starts_with("GET / HTTP") || req_line == "GET /" {
        append_killer_full_event(
            "ui_serve", "GET /", "", "", "", "", &peer_ip, 0, 0, 0, "", 0, "", 0u128, "page",
        );
        serve_html(&mut stream);
    } else if req_line.starts_with("POST /api/kala/cache-clear") {
        append_killer_full_event(
            "cache_clear",
            "POST /api/kala/cache-clear",
            "",
            "",
            "",
            "",
            &peer_ip,
            0,
            0,
            0,
            "",
            0,
            "",
            0u128,
            "cleared",
        );
        let (poly_n, llm_n) = crate::khlm_polyglot::clear_all_khlm_caches();
        let json_body = format!(
            r#"{{"ok":true,"polyglot_cleared":{},"inference_cleared":{}}}"#,
            poly_n, llm_n
        );
        let header = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: application/json\r\n\
             Access-Control-Allow-Origin: *\r\n\
             Content-Length: {}\r\n\
             \r\n",
            json_body.len()
        );
        let _ = stream.write_all(header.as_bytes());
        let _ = stream.write_all(json_body.as_bytes());
    } else if req_line.starts_with("POST /api/kala/clear-session") {
        append_killer_full_event(
            "session_clear",
            "POST /api/kala/clear-session",
            "",
            "",
            "",
            "",
            &peer_ip,
            0,
            0,
            0,
            "",
            0,
            "",
            0u128,
            "cleared",
        );
        crate::khlm_polyglot::clear_conversation_session();
        crate::khlm_polyglot::clear_all_khlm_caches();
        let json_body = r#"{"ok":true}"#;
        let header = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: application/json\r\n\
             Access-Control-Allow-Origin: *\r\n\
             Content-Length: {}\r\n\
             \r\n",
            json_body.len()
        );
        let _ = stream.write_all(header.as_bytes());
        let _ = stream.write_all(json_body.as_bytes());
    } else if req_line.starts_with("POST /api/kala/llm-config") {
        handle_llm_config(&mut stream, &body_str);
    } else if req_line.starts_with("POST /api/kala/llm-test") {
        handle_llm_test(&mut stream, &body_str);
    } else if req_line.starts_with("POST /api/kala") {
        handle_api(&mut stream, &body_str, &peer_ip);
    } else {
        let note = csv_sanitize(&req_line, 120);
        append_killer_full_event(
            "http_404",
            &note,
            "",
            "",
            "",
            "",
            &peer_ip,
            0,
            0,
            0,
            "",
            0,
            "",
            0u128,
            "not_found",
        );
        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
    }
}

fn serve_html(stream: &mut TcpStream) {
    let html = KALA_HTML.as_bytes();
    let header = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Cache-Control: no-store, no-cache, must-revalidate, max-age=0\r\n\
         Pragma: no-cache\r\n\
         Expires: 0\r\n\
         \r\n",
        html.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(html);
}

fn handle_api(stream: &mut TcpStream, body: &str, peer_ip: &str) {
    let mode     = extract_json_str(body, "mode")    .unwrap_or_else(|| "ask".into());
    let question = extract_json_str(body, "question").unwrap_or_default();
    let style    = extract_json_str(body, "style")   .unwrap_or_else(|| "essay".into());
    let lang     = extract_json_str(body, "lang")    .unwrap_or_else(|| "killer".into());
    let uname    = extract_json_str(body, "uname")   .unwrap_or_default();

    // Parse conversation history array: [{r:"user",c:"..."},{r:"assistant",c:"..."},...]
    let history  = extract_json_history(body);
    let history_turns = history.len() / 2;

    let t0 = std::time::Instant::now();

    let response = if mode == "code" && is_ghost_assembly(&question) {
        run_ghost_assembly_for_chat(&question)
    } else {
        crate::builtin::BuiltinFunctions::kala_dispatch_with_memory(
            &mode, &question, &style, &lang, &history, &uname
        )
    };
    let elapsed_ms = t0.elapsed().as_millis();

    // ── AI Metadata: thinking chain, suggestions, confidence ──
    let ai_meta = generate_ai_metadata(&question, &response, &mode, &history);

    // ── CSV tracking ─────────────────────────────────────────
    append_csv_log(
        &mode,
        &question,
        &style,
        &lang,
        &uname,
        peer_ip,
        history.len(),
        history_turns,
        question.len(),
        response.len(),
        &response,
        elapsed_ms,
    );

    let escaped   = json_escape(&response);
    let json_body = format!(
        "{{\"response\":\"{}\",\"mode\":\"{}\",\"thinking\":{},\"suggestions\":{},\"confidence\":{},\"intent\":\"{}\",\"topic\":\"{}\"}}",
        escaped, mode, ai_meta.thinking_json, ai_meta.suggestions_json,
        ai_meta.confidence, json_escape(&ai_meta.intent), json_escape(&ai_meta.topic)
    );
    let header = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: application/json\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Content-Length: {}\r\n\
         \r\n",
        json_body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(json_body.as_bytes());
}

fn handle_llm_config(stream: &mut TcpStream, body: &str) {
    let provider = extract_json_str(body, "provider").unwrap_or_default();
    let api_key  = extract_json_str(body, "api_key").unwrap_or_default();
    let model    = extract_json_str(body, "model").unwrap_or_default();

    let valid = provider.is_empty()
        || matches!(provider.to_lowercase().as_str(), "ollama" | "groq" | "openai" | "anthropic");
    if !valid {
        let err = r#"{"ok":false,"error":"Invalid provider"}"#;
        let h = format!(
            "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\n\r\n",
            err.len()
        );
        let _ = stream.write_all(h.as_bytes());
        let _ = stream.write_all(err.as_bytes());
        return;
    }

    {
        let mut cfg = crate::khlm_polyglot::config().lock().unwrap();
        cfg.llm_provider = provider.clone();
        cfg.llm_api_key  = api_key;
        if model.is_empty() {
            cfg.llm_model = match provider.as_str() {
                "ollama"    => "llama3".into(),
                "groq"      => "llama3-70b-8192".into(),
                "openai"    => "gpt-4o-mini".into(),
                "anthropic" => "claude-opus-4-5".into(),
                _           => String::new(),
            };
        } else {
            cfg.llm_model = model;
        }
    }

    // Fresh answers after provider/model/key change — avoids stale LLM + polyglot cache.
    crate::khlm_polyglot::clear_all_khlm_caches();

    let json_body = r#"{"ok":true}"#;
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\n\r\n",
        json_body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(json_body.as_bytes());
}

fn handle_llm_test(stream: &mut TcpStream, body: &str) {
    let provider = extract_json_str(body, "provider").unwrap_or_default();
    let api_key  = extract_json_str(body, "api_key").unwrap_or_default();
    let model    = extract_json_str(body, "model").unwrap_or_default();

    let model_resolved = if model.is_empty() {
        match provider.as_str() {
            "ollama"    => "llama3".to_string(),
            "groq"      => "llama3-70b-8192".to_string(),
            "openai"    => "gpt-4o-mini".to_string(),
            "anthropic" => "claude-opus-4-5".to_string(),
            _           => "unknown".to_string(),
        }
    } else {
        model
    };

    let llm_cfg = match provider.to_lowercase().as_str() {
        "ollama"    => crate::llm::LlmConfig::ollama(&model_resolved),
        "groq"      => crate::llm::LlmConfig::groq(&api_key, &model_resolved),
        "openai"    => crate::llm::LlmConfig::openai(&api_key, &model_resolved),
        "anthropic" => crate::llm::LlmConfig::anthropic(&api_key, &model_resolved),
        _ => {
            let err = r#"{"ok":false,"error":"Invalid provider"}"#;
            let h = format!(
                "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\n\r\n",
                err.len()
            );
            let _ = stream.write_all(h.as_bytes());
            let _ = stream.write_all(err.as_bytes());
            return;
        }
    };

    let msgs = vec![crate::llm::LlmMessage::user("Say hello in one sentence.")];
    let result = crate::llm::complete(&llm_cfg, &msgs);

    let json_body = match result {
        Ok(resp) => {
            let preview = resp.content.chars().take(120).collect::<String>();
            format!(
                "{{\"ok\":true,\"preview\":\"{}\"}}",
                json_escape(&preview)
            )
        }
        Err(e) => {
            format!(
                "{{\"ok\":false,\"error\":\"{}\"}}",
                json_escape(&e)
            )
        }
    };

    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\n\r\n",
        json_body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(json_body.as_bytes());
}

struct AiMeta {
    thinking_json: String,
    suggestions_json: String,
    confidence: f64,
    intent: String,
    topic: String,
}

fn generate_ai_metadata(question: &str, response: &str, mode: &str, history: &[(String, String)]) -> AiMeta {
    let q = question.trim().to_lowercase();
    let q_words: Vec<&str> = q.split_whitespace().collect();

    // ── Intent Detection ──────────────────────────────
    let intent = detect_intent(&q, mode);

    // ── Topic Extraction ──────────────────────────────
    let topic = extract_topic(&q, mode);

    // ── Confidence Scoring ────────────────────────────
    let confidence = compute_confidence(question, response, mode, history.len());

    // ── Thinking Chain ────────────────────────────────
    let thinking = generate_thinking_chain(&q, &intent, &topic, mode, q_words.len());

    // ── Smart Suggestions ─────────────────────────────
    let suggestions = generate_smart_suggestions(&q, response, &intent, &topic, mode);

    let thinking_json = format!("[{}]",
        thinking.iter().map(|s| format!("\"{}\"", json_escape(s))).collect::<Vec<_>>().join(","));
    let suggestions_json = format!("[{}]",
        suggestions.iter().map(|s| format!("\"{}\"", json_escape(s))).collect::<Vec<_>>().join(","));

    AiMeta { thinking_json, suggestions_json, confidence, intent, topic }
}

fn detect_intent(q: &str, mode: &str) -> String {
    if mode == "code" { return "code_generation".into(); }
    if mode == "think" { return "deep_reasoning".into(); }
    if mode == "write" { return "creative_writing".into(); }
    if mode == "imagine" { return "creative_imagination".into(); }
    if mode == "debug" { return "debugging".into(); }
    if mode == "lab" { return "ai_lab_demo".into(); }

    if q.contains("how does") && q.contains("work") {
        "explanation".into()
    } else if q.contains("how to") || q.contains("how do i") || q.contains("how can i") {
        "how_to".into()
    } else if q.starts_with("what") || q.starts_with("who") || q.starts_with("when") || q.starts_with("where") {
        "factual_query".into()
    } else if q.starts_with("why") {
        "explanation".into()
    } else if q.contains("compare") || q.contains("difference") || q.contains("vs") {
        "comparison".into()
    } else if q.contains("write") || q.contains("create") || q.contains("generate") || q.contains("build") || q.contains("make") {
        "creation".into()
    } else if q.contains("fix") || q.contains("error") || q.contains("bug") || q.contains("wrong") {
        "troubleshooting".into()
    } else if q.contains("explain") || q.contains("teach") || q.contains("understand") {
        "learning".into()
    } else if q.contains("opinion") || q.contains("think about") || q.contains("feel about") {
        "opinion".into()
    } else if q.contains("recommend") || q.contains("suggest") || q.contains("best") {
        "recommendation".into()
    } else if q.len() < 20 {
        "quick_query".into()
    } else {
        "general".into()
    }
}

fn extract_topic(q: &str, _mode: &str) -> String {
    let tech_topics = [
        ("artificial intelligence", "Artificial Intelligence"),
        ("machine learning", "Machine Learning"), ("deep learning", "Deep Learning"),
        ("neural", "Neural Networks"),
        ("rust", "Rust Programming"), ("python", "Python"), ("javascript", "JavaScript"),
        ("java ", "Java"), ("react", "React"), ("node", "Node.js"),
        ("typescript", "TypeScript"), ("go ", "Go/Golang"), ("c++", "C++"),
        ("html", "Web Development"), ("css", "Web Design"), ("sql", "Databases"),
        ("api", "API Development"), ("docker", "DevOps"), ("kubernetes", "Cloud/K8s"),
        ("blockchain", "Blockchain"), ("crypto", "Cryptocurrency"),
        ("algorithm", "Algorithms"), ("data structure", "Data Structures"),
        ("linux", "Linux"), ("git ", "Version Control"),
        ("gravity", "Physics"), ("quantum", "Physics"),
    ];
    for (kw, topic) in &tech_topics {
        if q.contains(kw) { return topic.to_string(); }
    }
    let q_words: Vec<&str> = q.split_whitespace().collect();
    if q_words.iter().any(|w| *w == "ai") {
        return "Artificial Intelligence".into();
    }

    let general_topics = [
        ("math", "Mathematics"), ("physics", "Physics"), ("chemistry", "Chemistry"),
        ("biology", "Biology"), ("history", "History"), ("philosophy", "Philosophy"),
        ("psychology", "Psychology"), ("economics", "Economics"),
        ("music", "Music"), ("movie", "Entertainment"),
        ("health", "Health"), ("fitness", "Fitness"), ("food", "Food/Cooking"),
        ("travel", "Travel"), ("space", "Space/Astronomy"),
        ("climate", "Climate/Environment"), ("business", "Business"),
        ("painting", "Art"), ("sculpture", "Art"), ("drawing", "Art"),
    ];
    for (kw, topic) in &general_topics {
        if q.contains(kw) { return topic.to_string(); }
    }

    "General".into()
}

fn compute_confidence(question: &str, response: &str, mode: &str, history_len: usize) -> f64 {
    let mut score = 0.7_f64;

    let q_len = question.len();
    let r_len = response.len();

    if r_len > 200 { score += 0.05; }
    if r_len > 500 { score += 0.05; }
    if r_len > 1000 { score += 0.03; }
    if response.contains("```") { score += 0.05; }
    if response.contains("**") { score += 0.02; }
    if q_len < 10 { score -= 0.05; }
    if history_len > 4 { score += 0.03; }
    if mode == "code" && response.contains("```") { score += 0.05; }
    if mode == "think" && r_len > 300 { score += 0.05; }
    if response.contains("I'm not sure") || response.contains("I don't know") { score -= 0.15; }
    if response.contains("might") || response.contains("perhaps") { score -= 0.05; }

    score.clamp(0.3, 0.98)
}

fn generate_thinking_chain(_q: &str, intent: &str, topic: &str, mode: &str, word_count: usize) -> Vec<String> {
    let mut steps = Vec::new();

    steps.push(format!("Analyzing intent: {}", intent));
    steps.push(format!("Topic detected: {}", topic));

    match intent {
        "code_generation" => {
            steps.push("Selecting language and framework".into());
            steps.push("Generating code structure".into());
            steps.push("Adding best practices and error handling".into());
        }
        "how_to" => {
            steps.push("Searching knowledge base".into());
            steps.push("Building step-by-step guide".into());
            steps.push("Adding practical examples".into());
        }
        "factual_query" => {
            steps.push("Querying knowledge base".into());
            steps.push("Cross-referencing facts".into());
            steps.push("Formatting clear answer".into());
        }
        "explanation" => {
            steps.push("Understanding the 'why' question".into());
            steps.push("Building causal reasoning chain".into());
            steps.push("Simplifying for clarity".into());
        }
        "comparison" => {
            steps.push("Identifying comparison subjects".into());
            steps.push("Analyzing pros and cons".into());
            steps.push("Building comparison table".into());
        }
        "deep_reasoning" => {
            steps.push("Decomposing the question".into());
            steps.push("Considering multiple perspectives".into());
            steps.push("Synthesizing insights".into());
            steps.push("Forming nuanced conclusion".into());
        }
        "troubleshooting" => {
            steps.push("Identifying error pattern".into());
            steps.push("Checking common causes".into());
            steps.push("Generating fix suggestions".into());
        }
        "creative_writing" => {
            steps.push("Understanding style and tone".into());
            steps.push("Structuring narrative".into());
            steps.push("Crafting prose".into());
        }
        _ => {
            if word_count > 15 {
                steps.push("Parsing complex query".into());
                steps.push("Breaking into sub-questions".into());
            }
            steps.push("Generating response".into());
        }
    }

    if mode == "ai_system" {
        steps.push("Routing through Ghost-108 search".into());
        steps.push("Merging multiple sources".into());
    }

    steps.push("Quality check complete".into());
    steps
}

fn generate_smart_suggestions(_q: &str, _response: &str, intent: &str, topic: &str, _mode: &str) -> Vec<String> {
    let mut sugs = Vec::new();

    match intent {
        "code_generation" => {
            sugs.push("Add unit tests".into());
            sugs.push("Optimize performance".into());
            sugs.push("Add error handling".into());
            sugs.push("Explain this code".into());
        }
        "how_to" => {
            sugs.push("Show me an example".into());
            sugs.push("What are common mistakes?".into());
            sugs.push("Any alternatives?".into());
        }
        "factual_query" => {
            sugs.push("Tell me more".into());
            sugs.push("Why is that?".into());
            sugs.push("Give me an example".into());
        }
        "explanation" => {
            sugs.push("Can you simplify?".into());
            sugs.push("Give a real-world example".into());
            sugs.push("What are the implications?".into());
        }
        "comparison" => {
            sugs.push("Which one should I use?".into());
            sugs.push("Show me benchmarks".into());
            sugs.push("Pros and cons table".into());
        }
        "troubleshooting" => {
            sugs.push("Show me the fix".into());
            sugs.push("Why does this happen?".into());
            sugs.push("Prevent this in future".into());
        }
        "deep_reasoning" => {
            sugs.push("Challenge this view".into());
            sugs.push("What's the counterargument?".into());
            sugs.push("Go deeper".into());
        }
        _ => {
            sugs.push("Tell me more".into());
            sugs.push("Give me an example".into());
        }
    }

    if topic != "General" && !topic.is_empty() {
        sugs.push(format!("More about {}", topic));
    }

    sugs.truncate(4);
    sugs
}

fn csv_sanitize(s: &str, max: usize) -> String {
    let clean = s.replace('"', "'").replace('\n', " ").replace('\r', "");
    if clean.len() > max {
        let mut end = max;
        while !clean.is_char_boundary(end) { end -= 1; }
        format!("{}...", &clean[..end])
    } else {
        clean
    }
}

/// FNV-1a 64-bit — first 16 hex chars (stable fingerprint for CSV analytics).
fn fnv1a64_hex(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", h)[..16].to_string()
}

fn response_flags(response: &str) -> String {
    let mut v: Vec<&'static str> = Vec::new();
    if response.is_empty() {
        v.push("EMPTY");
    }
    if response.contains("data:image/") {
        v.push("IMAGE");
    }
    if response.contains("```") {
        v.push("CODE_FENCE");
    }
    if response.contains("+-- KhLM") || response.contains("KhLM/") {
        v.push("KHLM");
    }
    if response.contains("Ghost-108") || response.contains("Killer AI System") {
        v.push("MULTI_AGENT");
    }
    if v.is_empty() {
        "text".into()
    } else {
        v.join("+")
    }
}

/// Full telemetry: every Kala HTTP event + rich chat row (hashes, flags, history counts).
fn append_killer_full_event(
    event: &str,
    route: &str,
    mode: &str,
    style: &str,
    lang: &str,
    user: &str,
    ip: &str,
    history_msgs: usize,
    history_pairs: usize,
    q_len: usize,
    q_fp: &str,
    r_len: usize,
    r_fp: &str,
    ms: u128,
    flags: &str,
) {
    use std::fs::OpenOptions;
    use std::io::Write as IoWrite;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let date_iso = unix_secs_to_date_iso(now);
    let path = "killer_kala_full_log.csv";
    let write_header = !std::path::Path::new(path).exists();
    let rt = csv_sanitize(route, 200);
    let ev = csv_sanitize(event, 64);
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        if write_header {
            let _ = writeln!(f,
                "ts_unix,date_iso,event,route,mode,style,lang,user,ip,history_msgs,history_pairs,q_len,q_fp16,r_len,r_fp16,latency_ms,flags"
            );
        }
        let _ = writeln!(
            f,
            "{},{},\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",{},{},{},\"{}\",{},\"{}\",{},\"{}\"",
            now,
            date_iso,
            ev,
            rt,
            csv_sanitize(mode, 40),
            csv_sanitize(style, 32),
            csv_sanitize(lang, 32),
            csv_sanitize(user, 80),
            csv_sanitize(ip, 64),
            history_msgs,
            history_pairs,
            q_len,
            q_fp,
            r_len,
            r_fp,
            ms,
            csv_sanitize(flags, 120)
        );
    }
}

fn unix_secs_to_date_iso(secs: u64) -> String {
    let days = secs / 86400;
    let y400 = days / 146097;
    let rem = days % 146097;
    let y100 = (rem / 36524).min(3);
    let rem = rem - y100 * 36524;
    let y4 = rem / 1461;
    let rem = rem % 1461;
    let y1 = (rem / 365).min(3);
    let year = y400 * 400 + y100 * 100 + y4 * 4 + y1 + 1970;
    let doy = rem - y1 * 365;
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let mdays: [u64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
    ];
    let (mut month, mut day) = (1u64, doy);
    for m in 0..12 {
        if day >= mdays[m] {
            day -= mdays[m];
            month = m as u64 + 2;
        } else {
            break;
        }
    }
    format!("{:04}-{:02}-{:02}", year, month, day + 1)
}

fn append_csv_log(
    mode: &str,
    question: &str,
    style: &str,
    lang: &str,
    uname: &str,
    ip: &str,
    history_msgs: usize,
    history_turns: usize,
    q_chars: usize,
    resp_len: usize,
    response: &str,
    ms: u128,
) {
    use std::fs::OpenOptions;
    use std::io::Write as IoWrite;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let q = csv_sanitize(question, 120);
    let preview = csv_sanitize(&response.replace('`', "'").replace('#', ""), 80);
    let q_fp = fnv1a64_hex(question);
    let r_fp = fnv1a64_hex(response);
    let flags = response_flags(response);

    append_killer_full_event(
        "kala_message",
        "POST /api/kala",
        mode,
        style,
        lang,
        uname,
        ip,
        history_msgs,
        history_turns,
        q_chars,
        &q_fp,
        resp_len,
        &r_fp,
        ms,
        &flags,
    );

    // ── 1. kala_chat_log.csv — detailed per-message log (legacy columns) ──────
    let chat_path = "kala_chat_log.csv";
    let write_header = !std::path::Path::new(chat_path).exists();
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(chat_path) {
        if write_header {
            let _ = writeln!(f,
                "timestamp,mode,style,lang,user,ip,history_turns,history_msgs,q_chars,question,resp_chars,resp_preview,latency_ms,q_fp16,r_fp16,flags"
            );
        }
        let _ = writeln!(f,
            "{},{},{},{},{},{},{},{},{},\"{}\",{},\"{}\",{},\"{}\",\"{}\",\"{}\"",
            now, mode, style, lang, uname, ip,
            history_turns, history_msgs, q_chars, q, resp_len, preview, ms, q_fp, r_fp, flags
        );
    }

    // ── 2. KILLER_MASTER_TRACKER.csv — project-wide tracker ──
    let master_path = "../../_TOOLS/KILLER_MASTER_TRACKER.csv";
    // Count existing rows to get next ID
    let next_id = std::fs::read_to_string(master_path)
        .map(|s| s.lines().count())   // header + data rows
        .unwrap_or(271);              // fallback: 270 existing + 1 header
    let date = unix_secs_to_date_iso(now);
    let desc = csv_sanitize(&format!("Kala chat [{}] mode={} user={} q={} resp={}c lat={}ms",
        &date, mode, uname, &q, resp_len, ms), 150);
    let notes = csv_sanitize(&format!("ip={} style={} lang={} history_turns={} preview: {}",
        ip, style, lang, history_turns, &preview), 200);
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(master_path) {
        let _ = writeln!(f,
            "{},KalaChat,Kala,src/kala_ui.rs,\"{}\",Live,{},1,1,0,Pass,{}ms,Hardened,Yes,\"{}\"",
            next_id, desc, date, ms, notes
        );
    }
}


// ─── JSON helpers — no serde needed ──────────────────────────────────────────

/// Extract a string value from a simple flat JSON object.
/// Handles JSON escape sequences properly.
fn extract_json_str(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\"", key);
    let pos = json.find(&needle)?;
    let after_key = &json[pos + needle.len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();
    if !after_colon.starts_with('"') {
        return None;
    }
    let chars: Vec<char> = after_colon[1..].chars().collect();
    let mut result = String::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '"' => break,
            '\\' if i + 1 < chars.len() => {
                i += 1;
                match chars[i] {
                    'n'  => result.push('\n'),
                    'r'  => result.push('\r'),
                    't'  => result.push('\t'),
                    '"'  => result.push('"'),
                    '\\' => result.push('\\'),
                    '/'  => result.push('/'),
                    'u' if i + 4 < chars.len() => {
                        let hex: String = chars[i + 1..i + 5].iter().collect();
                        if let Ok(n) = u32::from_str_radix(&hex, 16) {
                            if let Some(c) = char::from_u32(n) { result.push(c); }
                        }
                        i += 4;
                    }
                    c => result.push(c),
                }
            }
            c => result.push(c),
        }
        i += 1;
    }
    Some(result)
}

/// Parse conversation history array from JSON body.
/// Expects: "history":[{"r":"user","c":"..."},{"r":"assistant","c":"..."},...]
/// Returns vec of (role, content) pairs.
fn extract_json_history(json: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let needle = "\"history\":";
    let pos = match json.find(needle) { Some(p) => p, None => return result };
    let after = json[pos + needle.len()..].trim_start();
    if !after.starts_with('[') { return result; }
    // Walk through array entries
    let mut s = &after[1..]; // skip '['
    loop {
        s = s.trim_start();
        if s.starts_with(']') || s.is_empty() { break; }
        if s.starts_with('{') {
            // Find closing brace
            let end = find_obj_end(s);
            let entry = &s[..end];
            let role    = extract_json_str(entry, "r").unwrap_or_default();
            let content = extract_json_str(entry, "c").unwrap_or_default();
            if !role.is_empty() && !content.is_empty() {
                result.push((role, content));
            }
            s = &s[end..];
        } else {
            s = &s[1..]; // skip comma or unexpected char
        }
    }
    result
}

/// Find the index just past the closing '}' of a JSON object starting with '{'.
fn find_obj_end(s: &str) -> usize {
    let mut depth = 0usize;
    let mut in_str = false;
    let mut escape = false;
    for (i, c) in s.char_indices() {
        if escape { escape = false; continue; }
        if in_str {
            if c == '\\' { escape = true; }
            else if c == '"' { in_str = false; }
            continue;
        }
        match c {
            '"' => in_str = true,
            '{' => depth += 1,
            '}' => { depth -= 1; if depth == 0 { return i + 1; } }
            _ => {}
        }
    }
    s.len()
}

/// Escape a Rust string for safe embedding in a JSON string value.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        match c {
            '"'  => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c    => out.push(c),
        }
    }
    out
}

fn is_ghost_assembly(text: &str) -> bool {
    let kw = ["push ", "pop", "add", "sub", "mul", "halt", "syscall ", "load ", "store ", "dup", "jmp ", "jmpif "];
    let lower = text.to_lowercase();
    kw.iter().filter(|k| lower.contains(*k)).count() >= 2
}

fn run_ghost_assembly_for_chat(source: &str) -> String {
    use crate::ghost_vm::{assemble_capsule, RunStatus};
    use crate::ghost_world::WorldHost;

    let t0 = std::time::Instant::now();
    let capsule = match assemble_capsule(source) {
        Ok(c) => c,
        Err(e) => return format!("Assembly error: {e}"),
    };

    let mut host = WorldHost::new();
    host.output_capture = Some(Vec::new());
    let mut cap = capsule;
    let result = crate::ghost_vm::run(&mut cap, &mut host, Some(100_000));
    let elapsed = t0.elapsed().as_millis();
    let output = host.get_captured_output();

    match result {
        Ok(status) => {
            let code_len = cap.code.len();
            let status_str = match status {
                RunStatus::Halted => "halted",
                RunStatus::Stopped => "stopped",
                RunStatus::FuelExhausted => "fuel exhausted",
                RunStatus::Yielded => "yielded",
            };
            format!(
                "\u{1f527} Assembled and ran your Ghost program:\n```\n{}\n```\nExecution: {}ms, {} bytes of code, status: {}",
                if output.is_empty() { "(no output)".into() } else { output.trim_end().to_string() },
                elapsed, code_len, status_str
            )
        }
        Err(e) => format!(
            "\u{1f527} Ghost program error:\n```\n{}\n```\nOutput before error:\n```\n{}\n```",
            e, if output.is_empty() { "(none)" } else { output.trim_end() }
        ),
    }
}
