import { Component, OnInit } from '@angular/core';
import { ChartDataset, ChartOptions, ChartType } from 'chart.js';
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
      y: {
        min: 0,
      },
      x: {
        type: 'time',
        time: {
          displayFormats: {
            day: 'YYYY-MM-DD'
          }
        }
      }
    },
  };
  chartType: ChartType = 'line';

  constructor(private healthpiService: HealthpiService) {
  }

  updateData(records: Record[]) {
    this.chartData = [
      {
        label: 'Weight',
        data: records.map(record => record.values.weight ?? null)
      },
      {
        label: 'Fat',
        data: records.map(record => {
          if (record.values.weight && record.values?.fatPercent) {
            return record.values.weight * record.values?.fatPercent / 100.0
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
    this.healthpiService.getRecords(["Weight", "FatPercent"]).subscribe(records => this.updateData(records));
  }

}
