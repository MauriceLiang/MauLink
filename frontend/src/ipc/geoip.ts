import type { IpcClient } from './client';
export function createGeoIpApi(client: IpcClient) {
  return {
    get: () => client.call('geoip_database_get', {}),
    update: () => client.call('geoip_database_update', {}),
    delete: () => client.call('geoip_database_delete', {}),
    configure: (updateIntervalDays: number) => client.call('geoip_database_configure', { updateIntervalDays }),
    async import() {
      const file = await client.call('local_file_select', { purpose: 'geoIpDatabase' });
      return file ? client.call('geoip_database_import', { token: file.token }) : null;
    },
  };
}
export type GeoIpApi = ReturnType<typeof createGeoIpApi>;
