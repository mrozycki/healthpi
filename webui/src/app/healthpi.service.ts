import { HttpClient } from '@angular/common/http';
import { Injectable } from '@angular/core';

import { Record } from './records';

@Injectable({
  providedIn: 'root'
})
export class HealthpiService {
  constructor(private http: HttpClient) { }

  getRecords() {
    return this.http.get<Record[]>('http://localhost:8080/?select=Weight,FatPercent');
  }
}
