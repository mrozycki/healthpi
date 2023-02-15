import { TestBed } from '@angular/core/testing';
import { HttpClientModule } from '@angular/common/http';

import { HealthpiService } from './healthpi.service';

describe('HealthpiService', () => {
  let service: HealthpiService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [HttpClientModule]
    });
    service = TestBed.inject(HealthpiService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });
});
