import { ComponentFixture, TestBed } from '@angular/core/testing';
import { HttpClientModule } from '@angular/common/http';
import { HealthpiService } from '../healthpi.service';

import { WeightChartComponent } from './weight-chart.component';

describe('WeightChartComponent', () => {
  let component: WeightChartComponent;
  let fixture: ComponentFixture<WeightChartComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [WeightChartComponent],
      imports: [HttpClientModule],
      providers: [HealthpiService]
    })
      .compileComponents();

    fixture = TestBed.createComponent(WeightChartComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
