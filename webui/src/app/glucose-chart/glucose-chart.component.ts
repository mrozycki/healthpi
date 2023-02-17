import { Component, OnInit } from '@angular/core';
import { ChartDataset, ChartOptions, ChartType, ScatterDataPoint } from 'chart.js';
import { HealthpiService } from '../healthpi.service';
import { Record } from '../records';
import 'chartjs-adapter-moment';
import * as moment from 'moment';

@Component({
  selector: 'app-glucose-chart',
  templateUrl: './glucose-chart.component.html',
  styleUrls: ['./glucose-chart.component.less']
})
export class GlucoseChartComponent implements OnInit {

  chartData?: ChartDataset[];

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
    plugins: {
      colors: {
        enabled: true,
      }
    }
  };

  chartType: ChartType = 'scatter';

  constructor(private healthpiService: HealthpiService) {
  }

  updateData(records: Record[]) {
    const data = records
      .filter(record => record.values.glucose)
      .map(record => {
        const meal = record.values.meal ?? "NoIndication";
        const point: ScatterDataPoint = {
          x: moment(record.timestamp, "YYYY-MM-DD'T'HH:mm:ss").unix() * 1000,
          y: record.values.glucose ?? 0
        };
        return { meal: meal, point: point };
      });

    this.chartData = [
      {
        label: 'After fast',
        data: data
          .filter(record => record.meal == "NoMeal")
          .map(record => record.point),
      },
      {
        label: 'After meal',
        data: data
          .filter(record => record.meal == "AfterMeal")
          .map(record => record.point),
      },
      {
        label: 'Before meal',
        data: data
          .filter(record => record.meal == "BeforeMeal")
          .map(record => record.point),
      },
      {
        label: 'Other',
        data: data
          .filter(record => record.meal == "NoIndication")
          .map(record => record.point),
      },
    ];
  }

  ngOnInit(): void {
    this.healthpiService.getRecords(["Glucose", "Meal"]).subscribe(records => this.updateData(records));
  }

}
