import { COMPARISONS } from '../data/guideData';

export const ComparisonTable = () => (
  <div className="w-full rounded-xl border border-slate-700/80 bg-slate-900/50 shadow-xl overflow-x-auto">
    <table className="w-full text-left text-[13px] border-collapse">
      <thead>
        <tr className="bg-slate-800/80 border-b border-slate-700">
          <th className="p-4 font-mono font-medium text-slate-400 whitespace-nowrap">Capability</th>
          <th className="p-4 font-mono font-bold text-emerald-400 border-l border-slate-700/80 bg-emerald-950/20">
            avm
          </th>
          <th className="p-4 font-mono font-medium text-slate-400 border-l border-slate-700/80">asdf</th>
          <th className="p-4 font-mono font-medium text-slate-400 border-l border-slate-700/80">vfox</th>
          <th className="p-4 font-mono font-medium text-slate-400 border-l border-slate-700/80">mise</th>
          <th className="p-4 font-mono font-medium text-slate-400 border-l border-slate-700/80">proto</th>
        </tr>
      </thead>
      <tbody>
        {COMPARISONS.map((comp, idx) => (
          <tr key={idx} className="border-t border-slate-800/80 hover:bg-slate-800/30 transition-colors">
            <td className="p-4 font-mono text-slate-200 whitespace-nowrap align-top font-medium">
              {comp.feature}
            </td>
            <td className="p-4 text-slate-100 border-l border-slate-700/80 align-top font-medium bg-emerald-950/10">
              {comp.avm}
            </td>
            <td className="p-4 text-slate-400 border-l border-slate-700/80 align-top">{comp.asdf}</td>
            <td className="p-4 text-slate-400 border-l border-slate-700/80 align-top">{comp.vfox}</td>
            <td className="p-4 text-slate-400 border-l border-slate-700/80 align-top">{comp.mise}</td>
            <td className="p-4 text-slate-400 border-l border-slate-700/80 align-top">{comp.proto}</td>
          </tr>
        ))}
      </tbody>
    </table>
  </div>
);
