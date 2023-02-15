import { Component, OnInit } from '@angular/core';
import { ChartDataset, ChartOptions } from 'chart.js';
import { HealthpiService } from '../healthpi.service';
import { Record } from '../records';
import 'chartjs-adapter-moment';

@Component({
  selector: 'app-weight-chart',
  templateUrl: './weight-chart.component.html',
  styleUrls: ['./weight-chart.component.less']
})
export class WeightChartComponent implements OnInit {

  chartData?: ChartDataset[];

  chartLabels?: string[];

  chartOptions: ChartOptions = {
    responsive: true,
    scales: {
      yAxis: {
        min: 0,
      },
      xAxis: {
        type: 'time',
        time: {
          displayFormats: {
            day: 'YYYY-MM-DD'
          }
        }
      }
    },
  };
  chartType = 'line';

  constructor(private healthpiService: HealthpiService) {
  }

  updateData(records: Record[]) {
    this.chartData = [
      {
        label: 'Weight',
        data: records.map(record => record.values[0].Weight ?? null)
      },
      {
        label: 'Fat',
        data: records.map(record => {
          if (record.values[0].Weight && record.values[1]?.FatPercent) {
            return record.values[0].Weight * record.values[1]?.FatPercent / 100.0
          } else {
            return null;
          }
        }),
        spanGaps: true,
      }
    ];
    this.chartLabels = records.map(record => record.timestamp);
  }

  ngOnInit(): void {
    this.healthpiService.getRecords().subscribe(records => this.updateData(records));
  }

}
