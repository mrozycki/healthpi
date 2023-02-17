import { TestBed } from '@angular/core/testing';
import { AppComponent } from './app.component';
import { HttpClientModule } from '@angular/common/http';
import { WeightChartComponent } from './weight-chart/weight-chart.component';
import { GlucoseChartComponent } from './glucose-chart/glucose-chart.component';

describe('AppComponent', () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [HttpClientModule],
      declarations: [
        AppComponent,
        GlucoseChartComponent,
        WeightChartComponent,
      ],
    }).compileComponents();
  });

  it('should create the app', () => {
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;
    expect(app).toBeTruthy();
  });

  it(`should have as title 'webui'`, () => {
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;
    expect(app.title).toEqual('webui');
  });
});
