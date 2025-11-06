import { NgModule } from '@angular/core';
import { BrowserModule } from '@angular/platform-browser';
import { AppComponent } from './app.component';
import { ForgeConnectService } from '@mockforge/forgeconnect/adapters/angular';

@NgModule({
  declarations: [AppComponent],
  imports: [BrowserModule],
  providers: [
    {
      provide: ForgeConnectService,
      useFactory: () => {
        return new ForgeConnectService({
          mockMode: 'auto',
          autoMockStatusCodes: [404, 500],
          autoMockNetworkErrors: true,
        });
      },
    },
  ],
  bootstrap: [AppComponent],
})
export class AppModule {}
