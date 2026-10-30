import { COMPARISONS } from '../data/guideData';

export const ComparisonTable = () => (
  <div className="w-full rounded-lg border border-line/80 bg-surface/50 shadow-sm overflow-x-auto">
    <table className="w-full text-left text-sm border-collapse">
      <thead>
        <tr className="bg-muted/80 border-b border-line">
          <th className="p-4 font-mono font-medium text-secondary whitespace-nowrap">Capability</th>
          <th className="p-4 font-mono font-bold text-accent border-l border-line/80 bg-accent/20">
            avm
          </th>
          <th className="p-4 font-mono font-medium text-secondary border-l border-line/80">asdf</th>
          <th className="p-4 font-mono font-medium text-secondary border-l border-line/80">vfox</th>
          <th className="p-4 font-mono font-medium text-secondary border-l border-line/80">mise</th>
          <th className="p-4 font-mono font-medium text-secondary border-l border-line/80">proto</th>
        </tr>
      </thead>
      <tbody>
        {COMPARISONS.map((comp, idx) => (
          <tr key={idx} className="border-t border-line/80 hover:bg-muted/30 transition-colors">
            <td className="p-4 font-mono text-ink whitespace-nowrap align-top font-medium">
              {comp.feature}
            </td>
            <td className="p-4 text-ink border-l border-line/80 align-top font-medium bg-accent/10">
              {comp.avm}
            </td>
            <td className="p-4 text-secondary border-l border-line/80 align-top">{comp.asdf}</td>
            <td className="p-4 text-secondary border-l border-line/80 align-top">{comp.vfox}</td>
            <td className="p-4 text-secondary border-l border-line/80 align-top">{comp.mise}</td>
            <td className="p-4 text-secondary border-l border-line/80 align-top">{comp.proto}</td>
          </tr>
        ))}
      </tbody>
    </table>
  </div>
);
