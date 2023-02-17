import { ComponentFixture, TestBed } from '@angular/core/testing';
import { HttpClientModule } from '@angular/common/http';

import { HealthpiService } from '../healthpi.service';
import { GlucoseChartComponent } from './glucose-chart.component';

describe('GlucoseChartComponent', () => {
  let component: GlucoseChartComponent;
  let fixture: ComponentFixture<GlucoseChartComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [GlucoseChartComponent],
      imports: [HttpClientModule],
      providers: [HealthpiService]
    })
    .compileComponents();

    fixture = TestBed.createComponent(GlucoseChartComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
