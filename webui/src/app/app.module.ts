import { NgModule } from '@angular/core';
import { BrowserModule } from '@angular/platform-browser';
import { HttpClientModule } from '@angular/common/http';
import { NgChartsModule } from 'ng2-charts';

import { AppComponent } from './app.component';
import { WeightChartComponent } from './weight-chart/weight-chart.component';
import { GlucoseChartComponent } from './glucose-chart/glucose-chart.component';

@NgModule({
  declarations: [
    AppComponent,
    WeightChartComponent,
    GlucoseChartComponent
  ],
  imports: [
    BrowserModule,
    NgChartsModule,
    HttpClientModule
  ],
  providers: [],
  bootstrap: [AppComponent]
})
export class AppModule { }
