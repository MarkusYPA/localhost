/* global React, ReactDOM */
const { useState, useEffect, useRef } = React;

/* ============================================================
   KqueueDiagram — animated event-loop schematic
   - Listener accepts new connections
   - Connections move to FD pool, get registered with kqueue
   - kqueue emits READ / WRITE events back to the loop
   ============================================================ */
function KqueueDiagram() {
  const [t, setT] = useState(0);
  useEffect(() => {
    let raf;
    let start = performance.now();
    const tick = (now) => {
      setT(Math.max(0, (now - start) / 1000));
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, []);

  // simulate 5 connections cycling through stages
  const conns = [0,1,2,3,4].map(i => {
    const phase = (t * 0.5 + i * 0.45) % 4; // 0..4 stages
    return { i, phase };
  });

  // svg coords — total 1100 x 540
  const W = 1100, H = 540;
  const lanes = {
    accept: 80,    // left socket icon
    pool:   360,   // FD pool box
    kq:     680,   // kqueue
    loop:   980,   // event loop output
  };

  return (
    <div style={{width:'100%'}}>
      <svg viewBox={`0 0 ${W} ${H}`} style={{width:'100%', height:'auto', display:'block'}}>
        <defs>
          <marker id="arr" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto">
            <path d="M0,0 L10,5 L0,10 z" fill="#3D4A5E"/>
          </marker>
          <marker id="arrA" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto">
            <path d="M0,0 L10,5 L0,10 z" fill="#1E5BFF"/>
          </marker>
          <pattern id="grid" width="20" height="20" patternUnits="userSpaceOnUse">
            <path d="M 20 0 L 0 0 0 20" fill="none" stroke="#E1E6EE" strokeWidth="0.5"/>
          </pattern>
        </defs>

        <rect x="0" y="0" width={W} height={H} fill="url(#grid)"/>

        {/* lane labels */}
        <text x={lanes.accept} y="36" fontFamily="JetBrains Mono" fontSize="12" letterSpacing="2" fill="#6B7689">LISTENER :8080</text>
        <text x={lanes.pool}   y="36" fontFamily="JetBrains Mono" fontSize="12" letterSpacing="2" fill="#6B7689">FD POOL · MAX_CONN</text>
        <text x={lanes.kq}     y="36" fontFamily="JetBrains Mono" fontSize="12" letterSpacing="2" fill="#6B7689">KQUEUE()</text>
        <text x={lanes.loop}   y="36" fontFamily="JetBrains Mono" fontSize="12" letterSpacing="2" fill="#6B7689">EVENT LOOP</text>

        {/* listener box */}
        <g>
          <rect x={lanes.accept-30} y="60" width="120" height="80" fill="#fff" stroke="#0B1220" strokeWidth="1.5"/>
          <text x={lanes.accept+30} y="92" textAnchor="middle" fontFamily="JetBrains Mono" fontSize="14" fontWeight="600" fill="#0B1220">accept()</text>
          <text x={lanes.accept+30} y="115" textAnchor="middle" fontFamily="JetBrains Mono" fontSize="11" fill="#6B7689">SOCK_NONBLOCK</text>
          {/* pulse */}
          <circle cx={lanes.accept+30} cy="100" r={8 + (Math.sin(t*4)+1)*4} fill="none" stroke="#1E5BFF" strokeWidth="1" opacity={0.4 - (Math.sin(t*4)+1)*0.15}/>
        </g>

        {/* fd pool */}
        <g>
          <rect x={lanes.pool-90} y="60" width="180" height="420" fill="#fff" stroke="#0B1220" strokeWidth="1.5"/>
          <text x={lanes.pool} y="84" textAnchor="middle" fontFamily="JetBrains Mono" fontSize="13" fontWeight="600" fill="#0B1220">CONNECTIONS</text>
          <line x1={lanes.pool-90} y1="100" x2={lanes.pool+90} y2="100" stroke="#D4DAE5"/>
          {/* fd slots */}
          {[0,1,2,3,4,5,6,7].map(i => {
            const active = i < 5;
            const pulse = active ? 0.5 + 0.5 * Math.sin(t*1.5 + i) : 0;
            return (
              <g key={i}>
                <rect x={lanes.pool-72} y={114 + i*42} width="144" height="32" fill={active ? "#F4F6FA" : "#fff"} stroke="#D4DAE5"/>
                <text x={lanes.pool-60} y={134 + i*42} fontFamily="JetBrains Mono" fontSize="13" fill={active ? "#0B1220" : "#A6B0C2"}>
                  fd:{String(7+i).padStart(2,'0')}
                </text>
                {active && (
                  <>
                    <circle cx={lanes.pool+50} cy={130 + i*42} r="4" fill="#1E5BFF" opacity={0.3 + pulse*0.5}/>
                    <text x={lanes.pool+12} y={134 + i*42} fontFamily="JetBrains Mono" fontSize="11" fill="#3D4A5E">
                      {['READ','WRITE','READ','READ','WRITE'][i]}
                    </text>
                  </>
                )}
              </g>
            );
          })}
        </g>

        {/* kqueue */}
        <g>
          <rect x={lanes.kq-80} y="160" width="160" height="220" fill="#0B1220" stroke="#0B1220"/>
          <text x={lanes.kq} y="195" textAnchor="middle" fontFamily="JetBrains Mono" fontSize="16" fontWeight="600" fill="#fff">kqueue()</text>
          <text x={lanes.kq} y="218" textAnchor="middle" fontFamily="JetBrains Mono" fontSize="11" fill="#A6B0C2">EVENT QUEUE</text>
          <line x1={lanes.kq-60} y1="234" x2={lanes.kq+60} y2="234" stroke="rgba(255,255,255,0.2)"/>
          {/* events bubbling up */}
          {[0,1,2,3].map(i => {
            const y = 360 - ((t * 30 + i * 35) % 120);
            const op = ['EVFILT_READ','EVFILT_WRITE','EVFILT_READ','EVFILT_WRITE'][i];
            return (
              <g key={i} opacity={0.4 + 0.6 * Math.min(1, (360-y)/120)}>
                <rect x={lanes.kq-60} y={y} width="120" height="22" fill="rgba(30,91,255,0.2)" stroke="#1E5BFF" strokeWidth="0.5"/>
                <text x={lanes.kq} y={y+15} textAnchor="middle" fontFamily="JetBrains Mono" fontSize="10" fill="#fff">{op}</text>
              </g>
            );
          })}
        </g>

        {/* event loop */}
        <g>
          <rect x={lanes.loop-60} y="160" width="120" height="220" fill="#fff" stroke="#0B1220" strokeWidth="1.5"/>
          <text x={lanes.loop} y="195" textAnchor="middle" fontFamily="JetBrains Mono" fontSize="14" fontWeight="600" fill="#0B1220">dispatch</text>
          <text x={lanes.loop} y="216" textAnchor="middle" fontFamily="JetBrains Mono" fontSize="11" fill="#6B7689">handler()</text>
          {/* spinning indicator */}
          <g transform={`translate(${lanes.loop} 290) rotate(${(t*60)%360})`}>
            <circle cx="0" cy="0" r="28" fill="none" stroke="#D4DAE5"/>
            <circle cx="0" cy="0" r="28" fill="none" stroke="#1E5BFF" strokeWidth="2" strokeDasharray="40 200"/>
            <circle cx="28" cy="0" r="3" fill="#1E5BFF"/>
          </g>
          <text x={lanes.loop} y="358" textAnchor="middle" fontFamily="JetBrains Mono" fontSize="10" letterSpacing="2" fill="#6B7689">O(1) RETRIEVAL</text>
        </g>

        {/* arrows */}
        <line x1={lanes.accept+90} y1="100" x2={lanes.pool-90} y2="100" stroke="#3D4A5E" strokeWidth="1.2" markerEnd="url(#arr)"/>
        <text x={(lanes.accept+lanes.pool)/2} y="93" textAnchor="middle" fontFamily="JetBrains Mono" fontSize="10" fill="#3D4A5E">register</text>

        <line x1={lanes.pool+90} y1="270" x2={lanes.kq-80} y2="270" stroke="#3D4A5E" strokeWidth="1.2" markerEnd="url(#arr)"/>
        <text x={(lanes.pool+lanes.kq)/2} y="262" textAnchor="middle" fontFamily="JetBrains Mono" fontSize="10" fill="#3D4A5E">EV_SET</text>

        <line x1={lanes.kq+80} y1="270" x2={lanes.loop-60} y2="270" stroke="#1E5BFF" strokeWidth="1.5" markerEnd="url(#arrA)"/>
        <text x={(lanes.kq+lanes.loop)/2} y="262" textAnchor="middle" fontFamily="JetBrains Mono" fontSize="10" fill="#1E5BFF">kevent()</text>

        {/* return path: loop → pool */}
        <path d={`M ${lanes.loop-60} 320 Q ${(lanes.loop+lanes.pool)/2} 460 ${lanes.pool+90} 380`} fill="none" stroke="#3D4A5E" strokeDasharray="4 3" strokeWidth="1" markerEnd="url(#arr)"/>
        <text x={(lanes.loop+lanes.pool)/2} y="478" textAnchor="middle" fontFamily="JetBrains Mono" fontSize="10" fill="#3D4A5E">read() / write()</text>

        {/* moving packet on accept→pool path */}
        {[0,1].map(i => {
          const p = ((t*0.6 + i*0.5) % 1);
          const x = lanes.accept+90 + (lanes.pool-90 - (lanes.accept+90)) * p;
          return <rect key={i} x={x-5} y="96" width="10" height="8" fill="#1E5BFF" opacity={1-p}/>;
        })}

        {/* moving packets on kq→loop */}
        {[0,1,2].map(i => {
          const p = ((t*0.8 + i*0.4) % 1);
          const x = lanes.kq+80 + (lanes.loop-60 - (lanes.kq+80)) * p;
          return <circle key={i} cx={x} cy="270" r="4" fill="#1E5BFF" opacity={1-p}/>;
        })}

        {/* readout strip */}
        <g transform="translate(0 500)">
          <line x1="0" y1="0" x2={W} y2="0" stroke="#D4DAE5"/>
          <text x="20" y="22" fontFamily="JetBrains Mono" fontSize="12" fill="#3D4A5E">
            <tspan fill="#6B7689">CONN ACTIVE </tspan>
            <tspan fill="#0B1220" fontWeight="600">{conns.length.toString().padStart(3,'0')}</tspan>
            <tspan fill="#6B7689" dx="40">EVENTS/S </tspan>
            <tspan fill="#0B1220" fontWeight="600">{Math.floor(800 + Math.sin(t*0.7)*120)}</tspan>
            <tspan fill="#6B7689" dx="40">P95 LATENCY </tspan>
            <tspan fill="#0B1220" fontWeight="600">0.{Math.floor(8+Math.sin(t)*1)}ms</tspan>
            <tspan fill="#6B7689" dx="40">UPTIME </tspan>
            <tspan fill="#0B1220" fontWeight="600">{Math.floor(t).toString().padStart(5,'0')}s</tspan>
          </text>
          <circle cx={W-20} cy="14" r="4" fill="#1E5BFF" opacity={0.5 + 0.5*Math.sin(t*4)}/>
          <text x={W-32} y="18" textAnchor="end" fontFamily="JetBrains Mono" fontSize="11" fill="#1E5BFF">LIVE</text>
        </g>
      </svg>
    </div>
  );
}

/* ============================================================
   RequestFlowDiagram — animated request lifecycle
   stages: ACCEPT → INGEST → PARSE → DISPATCH → DELIVER
   ============================================================ */
function RequestFlowDiagram() {
  const [t, setT] = useState(0);
  useEffect(() => {
    let raf;
    let start = performance.now();
    const tick = (now) => {
      setT(Math.max(0, (now - start) / 1000));
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, []);

  const stages = [
    { k:'01', n:'ACCEPT',   d:'kqueue listener identifies SYN, registers FD' },
    { k:'02', n:'INGEST',   d:'non-blocking read() fills per-session buffer' },
    { k:'03', n:'PARSE',    d:'state machine: request line → headers → body' },
    { k:'04', n:'DISPATCH', d:'host header → vhost → handler / CGI / static' },
    { k:'05', n:'DELIVER',  d:'response buffered; flush on EVFILT_WRITE' },
  ];

  // viewBox has left/right padding so the first stage's x-90 anchor isn't clipped
  const PAD = 110;
  const W = 1500 + PAD*2, H = 360;
  const colW = (W - PAD*2 - 40) / stages.length;

  // active stage cycles - ensure non-negative for safety
  const activeIdx = Math.max(0, Math.floor(t * 1.2)) % stages.length;
  const subPhase  = (t * 1.2) % 1;

  return (
    <div style={{width:'100%'}}>
      <svg viewBox={`0 0 ${W} ${H}`} style={{width:'100%', height:'auto', display:'block'}}>
        <defs>
          <marker id="arr2" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto">
            <path d="M0,0 L10,5 L0,10 z" fill="#1E5BFF"/>
          </marker>
        </defs>

        {/* axis */}
        <line x1={PAD+20} y1="240" x2={W-PAD-20} y2="240" stroke="#D4DAE5"/>
        {[0,1,2,3,4].map(i => (
          <g key={i}>
            <line x1={PAD + 40 + i*colW} y1="234" x2={PAD + 40 + i*colW} y2="246" stroke="#A6B0C2"/>
            <text x={PAD + 40 + i*colW} y="266" textAnchor="middle" fontFamily="JetBrains Mono" fontSize="11" fill="#6B7689">t{i}</text>
          </g>
        ))}

        {/* stages */}
        {stages.map((s, i) => {
          const x = PAD + 40 + i*colW;
          const isActive = i === activeIdx;
          const isPast = i < activeIdx;
          return (
            <g key={i}>
              <rect x={x-90} y="40" width="200" height="160"
                    fill={isActive ? "#0B1220" : "#fff"}
                    stroke={isActive ? "#0B1220" : "#0B1220"}
                    strokeWidth="1.5"
                    opacity={isPast ? 0.55 : 1}/>
              {/* corner num */}
              <rect x={x-90} y="40" width="40" height="22"
                    fill={isActive ? "#1E5BFF" : "#0B1220"}/>
              <text x={x-70} y="56" textAnchor="middle" fontFamily="JetBrains Mono" fontSize="12" fill="#fff" fontWeight="600">{s.k}</text>

              <text x={x+10} y="92" textAnchor="middle" fontFamily="JetBrains Mono" fontSize="20" fontWeight="700"
                    fill={isActive ? "#fff" : "#0B1220"}
                    letterSpacing="2">{s.n}</text>

              <foreignObject x={x-80} y="108" width="180" height="80">
                <div style={{
                  fontFamily:'JetBrains Mono, monospace',
                  fontSize:'12px',
                  lineHeight:'1.5',
                  color: isActive ? 'rgba(255,255,255,0.85)' : '#3D4A5E',
                  fontWeight: 300
                }}>{s.d}</div>
              </foreignObject>

              {/* progress on active */}
              {isActive && (
                <rect x={x-90} y="198" width={Math.max(0, 200 * subPhase)} height="2" fill="#1E5BFF"/>
              )}
              {/* led */}
              <circle cx={x+95} cy="51" r="4"
                fill={isActive ? "#1E5BFF" : isPast ? "#1E5BFF" : "#D4DAE5"}
                opacity={isActive ? 0.5 + 0.5*Math.sin(t*8) : 1}/>
            </g>
          );
        })}

        {/* arrows between stages */}
        {stages.slice(0,-1).map((_, i) => {
          const x = PAD + 40 + i*colW + 110;
          return (
            <line key={i} x1={x} y1="120" x2={x + colW - 200} y2="120"
                  stroke="#1E5BFF" strokeWidth="1.5" markerEnd="url(#arr2)" opacity={i < activeIdx ? 1 : 0.25}/>
          );
        })}

        {/* moving packet */}
        {(() => {
          const startX = PAD + 40 + activeIdx*colW - 90;
          const x = startX + 200 * subPhase + (subPhase > 0.95 ? 40 : 0);
          return (
            <g>
              <rect x={x-6} y="218" width="12" height="6" fill="#1E5BFF"/>
              <text x={x} y="296" textAnchor="middle" fontFamily="JetBrains Mono" fontSize="10" fill="#1E5BFF">▲ req-{Math.floor(t*7).toString(16).padStart(4,'0')}</text>
            </g>
          );
        })()}

        {/* timing readout */}
        <g transform={`translate(${PAD+20} 320)`}>
          <text fontFamily="JetBrains Mono" fontSize="11" fill="#6B7689" letterSpacing="2">
            <tspan>BUDGET 10ms</tspan>
            <tspan dx="30" fill="#0B1220">USED {(subPhase*2).toFixed(1)}ms</tspan>
            <tspan dx="30" fill="#0B1220">STAGE {stages[activeIdx]?.n || ''}</tspan>
            <tspan dx="30" fill="#1E5BFF">● TRANSMITTING</tspan>
          </text>
        </g>
      </svg>
    </div>
  );
}

/* ============================================================
   CIPipeline — small animated gh-actions style strip
   ============================================================ */
function CIPipeline() {
  const [t, setT] = useState(0);
  useEffect(() => {
    let raf;
    const start = performance.now();
    const tick = (now) => { setT(Math.max(0, (now - start)/1000)); raf = requestAnimationFrame(tick); };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, []);

  const jobs = [
    { n:'checkout',  d:1.2 },
    { n:'cargo build', d:3.8 },
    { n:'clippy',    d:1.6 },
    { n:'cargo test',d:4.2 },
    { n:'integration', d:3.4 },
    { n:'report',    d:0.8 },
  ];
  const total = jobs.reduce((a,b) => a+b.d, 0);
  const cycle = (t * 0.6) % (total + 2); // pause at end

  return (
    <div style={{width:'100%'}}>
      <svg viewBox="0 0 1280 320" style={{width:'100%', height:'auto', display:'block'}}>
        {/* header */}
        <rect x="0" y="0" width="1280" height="48" fill="#0B1220"/>
        <circle cx="24" cy="24" r="6" fill="#1E5BFF" opacity={0.5+0.5*Math.sin(t*4)}/>
        <text x="40" y="29" fontFamily="JetBrains Mono" fontSize="14" fill="#fff" letterSpacing="2">.GITHUB / WORKFLOWS / CI.YML</text>
        <text x="1260" y="29" textAnchor="end" fontFamily="JetBrains Mono" fontSize="12" fill="rgba(255,255,255,0.5)" letterSpacing="2">RUN #4{Math.floor(t).toString().padStart(2,'0')}</text>

        {/* timeline */}
        <line x1="20" y1="120" x2="1260" y2="120" stroke="#D4DAE5"/>

        {(() => {
          let acc = 0;
          return jobs.map((j, i) => {
            const start = acc;
            acc += j.d;
            const xs = 20 + (start / total) * 1240;
            const xw = (j.d / total) * 1240;
            const done = cycle >= acc;
            const active = cycle > start && cycle < acc;
            const fill = done ? "#1E5BFF" : active ? "#0B1220" : "#fff";
            const stroke = done ? "#1E5BFF" : "#0B1220";
            const textColor = done || active ? "#fff" : "#0B1220";

            return (
              <g key={i}>
                <rect x={xs} y="92" width={xw - 6} height="56" fill={fill} stroke={stroke} strokeWidth="1.2"/>
                <text x={xs+12} y="115" fontFamily="JetBrains Mono" fontSize="13" fontWeight="600" fill={textColor}>{j.n}</text>
                <text x={xs+12} y="135" fontFamily="JetBrains Mono" fontSize="11" fill={done || active ? "rgba(255,255,255,0.7)" : "#6B7689"}>{j.d.toFixed(1)}s</text>
                {/* progress on active */}
                {active && (
                  <rect x={xs} y="146" width={Math.max(0, (xw-6) * ((cycle - start)/j.d))} height="2" fill="#1E5BFF"/>
                )}
                {/* status tick */}
                {done && (
                  <text x={xs + xw - 22} y="115" fontFamily="JetBrains Mono" fontSize="13" fill="#fff">✓</text>
                )}
              </g>
            );
          });
        })()}

        {/* lower stats */}
        <g transform="translate(20 200)">
          <rect x="0" y="0" width="1240" height="100" fill="#F4F6FA" stroke="#D4DAE5"/>
          <g transform="translate(20 28)">
            <text fontFamily="JetBrains Mono" fontSize="11" fill="#6B7689" letterSpacing="2">EVENT</text>
            <text y="28" fontFamily="JetBrains Mono" fontSize="20" fontWeight="600" fill="#0B1220">push → main</text>
          </g>
          <g transform="translate(280 28)">
            <text fontFamily="JetBrains Mono" fontSize="11" fill="#6B7689" letterSpacing="2">RUNNER</text>
            <text y="28" fontFamily="JetBrains Mono" fontSize="20" fontWeight="600" fill="#0B1220">macos-14 · arm64</text>
          </g>
          <g transform="translate(620 28)">
            <text fontFamily="JetBrains Mono" fontSize="11" fill="#6B7689" letterSpacing="2">TOOLCHAIN</text>
            <text y="28" fontFamily="JetBrains Mono" fontSize="20" fontWeight="600" fill="#0B1220">rust 1.78 · stable</text>
          </g>
          <g transform="translate(940 28)">
            <text fontFamily="JetBrains Mono" fontSize="11" fill="#6B7689" letterSpacing="2">STATUS</text>
            <g transform="translate(0 12)">
              <rect x="0" y="0" width="14" height="14" fill="#1E5BFF"/>
              <text x="22" y="12" fontFamily="JetBrains Mono" fontSize="20" fontWeight="600" fill="#0B1220">PASSING</text>
            </g>
          </g>
          <text x="1228" y="92" textAnchor="end" fontFamily="JetBrains Mono" fontSize="11" fill="#6B7689" letterSpacing="2">
            ELAPSED {cycle.toFixed(1)}s / {total.toFixed(1)}s
          </text>
        </g>
      </svg>
    </div>
  );
}

/* ============================================================
   Mount diagrams to elements with [data-diagram]
   ============================================================ */
function mountDiagrams() {
  document.querySelectorAll('[data-diagram="kqueue"]').forEach(el => {
    if (el.dataset.mounted) return;
    el.dataset.mounted = '1';
    ReactDOM.createRoot(el).render(<KqueueDiagram />);
  });
  document.querySelectorAll('[data-diagram="request-flow"]').forEach(el => {
    if (el.dataset.mounted) return;
    el.dataset.mounted = '1';
    ReactDOM.createRoot(el).render(<RequestFlowDiagram />);
  });
  document.querySelectorAll('[data-diagram="ci"]').forEach(el => {
    if (el.dataset.mounted) return;
    el.dataset.mounted = '1';
    ReactDOM.createRoot(el).render(<CIPipeline />);
  });
}

window.addEventListener('DOMContentLoaded', mountDiagrams);
// also re-mount when slides change in case React got torn down
document.addEventListener('slidechange', mountDiagrams);
