import { useId } from "react";
import type { LucideIcon } from "lucide-react";
import type { OperationalStatus } from "../types";

export interface HolotableSystem {
  id: string;
  label: string;
  metric: string;
  status: OperationalStatus;
  icon: LucideIcon;
}

// Elliptical coordinates preserve the reference's tilted projection while
// keeping labels upright and interaction targets in the same SVG space.
function point(radius: number, degrees: number) {
  const radians = degrees * Math.PI / 180;
  return [380 + Math.cos(radians) * radius, 263 + Math.sin(radians) * radius * .76];
}

function sector(inner: number, outer: number, start: number, end: number) {
  const a = point(outer, start), b = point(outer, end);
  const c = point(inner, end), d = point(inner, start);
  return `M ${a.join(" ")} A ${outer} ${outer * .76} 0 0 1 ${b.join(" ")} L ${c.join(" ")} A ${inner} ${inner * .76} 0 0 0 ${d.join(" ")} Z`;
}

export function SystemHolotable({ systems, selected, onSelect }: {
  systems: HolotableSystem[];
  selected: string;
  onSelect: (id: string) => void;
}) {
  const id = useId().replace(/:/g, "");
  const fill = (name: string) => `url(#${id}-${name})`;
  return <svg className="holo-projection" viewBox="0 0 760 575" role="group" aria-label="Interactive system holotable">
    <defs>
      <radialGradient id={`${id}-blue`} cx="50%" cy="47%" r="64%">
        <stop offset="0" stopColor="#16405b" stopOpacity=".62" />
        <stop offset=".63" stopColor="#366b87" stopOpacity=".58" />
        <stop offset="1" stopColor="#77bedc" stopOpacity=".48" />
      </radialGradient>
      <radialGradient id={`${id}-green`} cx="62%" cy="40%" r="70%">
        <stop stopColor="#145859" stopOpacity=".8" />
        <stop offset=".74" stopColor="#18af94" stopOpacity=".76" />
        <stop offset="1" stopColor="#83ffe0" stopOpacity=".92" />
      </radialGradient>
      <radialGradient id={`${id}-floor`}>
        <stop stopColor="#71d6ff" stopOpacity=".42" /><stop offset=".5" stopColor="#36b5e4" stopOpacity=".17" /><stop offset="1" stopColor="#103a51" stopOpacity="0" />
      </radialGradient>
      <linearGradient id={`${id}-beam`} x1="0" y1="0" x2="0" y2="1">
        <stop stopColor="#58bfe6" stopOpacity="0" /><stop offset="1" stopColor="#57d6ff" stopOpacity=".12" />
      </linearGradient>
      <pattern id={`${id}-grid`} width="18" height="14" patternUnits="userSpaceOnUse">
        <path d="M18 0H0V14" fill="none" stroke="#a5dcf2" strokeWidth=".45" opacity=".24" />
      </pattern>
      <filter id={`${id}-glow`} x="-35%" y="-35%" width="170%" height="170%">
        <feGaussianBlur stdDeviation="4" />
      </filter>
    </defs>

    <g aria-hidden="true" pointerEvents="none">
      <ellipse cx="380" cy="525" rx="367" ry="55" fill={fill("floor")} />
      <path d="M90 338 205 530 555 530 670 338Z" fill={fill("beam")} />
      {[0, 1, 2, 3].map(index => <ellipse key={index} cx="380" cy={523 + index * 3} rx={327 - index * 18} ry={38 - index * 5} fill="none" stroke="#5bb1d8" strokeOpacity={.1 + index * .045} strokeWidth={index === 0 ? 5 : 1.5} />)}
      <ellipse cx="380" cy="526" rx="266" ry="23" fill="none" stroke="#82ddfa" strokeWidth="5" strokeDasharray="2 9" opacity=".3" />
      {[321, 336, 354, 365].map((radius, index) => <ellipse key={radius} cx="380" cy="263" rx={radius} ry={radius * .76} fill="none" stroke="#67afd2" strokeWidth={index === 2 ? 7 : 1} strokeDasharray={index === 2 ? "30 88 7 33" : undefined} opacity={index === 2 ? .22 : .34} />)}
      {Array.from({ length: 120 }, (_, index) => {
        const angle = index * 3;
        const a = point(index % 5 === 0 ? 331 : 342, angle), b = point(350, angle);
        return <path key={index} d={`M${a.join(" ")} L${b.join(" ")}`} stroke="#8ad8f3" strokeWidth={index % 5 === 0 ? 1.5 : .8} opacity={index % 5 === 0 ? .55 : .23} />;
      })}
      {[0, 60, 120].map(angle => {
        const a = point(370, angle), b = point(370, angle + 180);
        return <path key={angle} d={`M${a.join(" ")} L${b.join(" ")}`} stroke="#6ec6ec" opacity=".16" strokeDasharray="5 5" />;
      })}
    </g>

    {systems.map((system, index) => {
      const start = -118 + index * 60, end = -62 + index * 60;
      const shape = sector(116, 302, start, end);
      const chosen = selected === system.id;
      const [x, y] = point(211, -90 + index * 60);
      const Icon = system.icon;
      return <g key={system.id} className={`holo-sector ${system.status}${chosen ? " selected" : ""}`} role="button" tabIndex={0}
        aria-label={`${system.label}: ${system.status}. ${system.metric}`} aria-pressed={chosen}
        onClick={() => onSelect(system.id)} onKeyDown={event => {
          if (event.key === "Enter" || event.key === " ") { event.preventDefault(); onSelect(system.id); }
        }}>
        <path className="sector-focus" d={sector(110, 308, start - .6, end + .6)} fill="none" />
        {chosen && <path d={shape} fill="none" stroke="#4cfdd2" strokeWidth="10" opacity=".6" filter={fill("glow")} aria-hidden="true" />}
        <path className="sector-surface" d={shape} fill={fill(chosen ? "green" : "blue")} stroke={chosen ? "#b2ffe9" : "#85c5e2"} strokeWidth={chosen ? 2.5 : 1.5} />
        <g aria-hidden="true" pointerEvents="none">
          <path d={shape} fill={fill("grid")} />
          {[126, 137, 149, 267, 280, 292].map(radius => <path key={radius} d={sector(radius, radius + .6, start, end)} fill={chosen ? "#8dffe0" : "#a0d9ef"} opacity=".24" />)}
          <path d={sector(124, 295, start + 1.1, end - 1.1)} fill="none" stroke={chosen ? "#adffe7" : "#a0d9ef"} opacity=".65" />
        </g>
        <g className="sector-label" transform={`translate(${x} ${y})`} pointerEvents="none" aria-hidden="true">
          <Icon x={-17} y={-44} width={34} height={34} strokeWidth={1.8} />
          <text textAnchor="middle" y="10" className="sector-name">{system.label.toUpperCase()}</text>
          <text textAnchor="middle" y="32" className={`sector-status ${system.status}`}>{system.status === "ready" ? "●  OK" : `●  ${system.status.toUpperCase()}`}</text>
          <text textAnchor="middle" y="53" className="sector-metric">{system.metric}</text>
        </g>
      </g>;
    })}

    <g className="holo-hub" aria-hidden="true" pointerEvents="none">
      {[97, 89, 75, 61, 55].map((radius, index) => <ellipse key={radius} cx="380" cy="263" rx={radius} ry={radius * .76} fill={index === 0 ? "#071b29" : "none"} fillOpacity=".74" stroke="#64d8ff" strokeWidth={index === 1 ? 4 : .8} strokeDasharray={index === 1 ? "51 42 18 34" : undefined} opacity={index === 1 ? .85 : .42} />)}
      <path d="M272 263H488M380 176V349" stroke="#8addfb" opacity=".45" strokeDasharray="3 7" />
      <circle cx="380" cy="263" r="3" fill="#a4ecff" />
      <path d="M374 254H368V249M386 272H392V277" fill="none" stroke="#9be7ff" opacity=".7" />
    </g>
  </svg>;
}
