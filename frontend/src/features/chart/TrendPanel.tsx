import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { StatusChip } from '@/shared/components/StatusChip';
import { displayUnit } from '@/shared/domain';
import { showTrendPrompt, useAnalyzer } from '@/state';

import { ChartLegend } from './ChartLegend';
import { DataTable } from './DataTable';
import { SignalList } from './SignalList';
import { TrendChart } from './TrendChart';

/** The selected biomarker: chart or table, legend, and the signals behind its status. */
export function TrendPanel() {
  const series = useAnalyzer((s) => s.series);
  const prompt = useAnalyzer(showTrendPrompt);
  if (!series) {
    return prompt ? (
      <p className="rounded-lg border border-dashed border-border p-6 text-center text-sm text-muted-foreground">
        Select a biomarker card to see its trend.
      </p>
    ) : null;
  }
  const { report } = series;
  const name = report.analyte?.display ?? series.code;

  return (
    <Card aria-label={`${name} trend`} role="region">
      <CardHeader className="flex flex-row flex-wrap items-center justify-between gap-2">
        <CardTitle>
          <h3 className="text-base">
            {series.code} ({displayUnit(report.unit)}) · {report.points.length} readings
          </h3>
        </CardTitle>
        <StatusChip status={report.status} />
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <Tabs defaultValue="chart">
          <TabsList aria-label="Trend view">
            <TabsTrigger value="chart">Chart</TabsTrigger>
            <TabsTrigger value="table">Table</TabsTrigger>
          </TabsList>
          <TabsContent value="chart" className="flex flex-col gap-3 pt-2">
            <TrendChart report={report} label={`${name} trend`} />
            <ChartLegend report={report} />
          </TabsContent>
          <TabsContent value="table" className="pt-2">
            <DataTable series={series} label={`${name} results`} />
          </TabsContent>
        </Tabs>
        <SignalList report={report} />
      </CardContent>
    </Card>
  );
}
