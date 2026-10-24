# Certificacion fisica de red kernel v1

Estado normativo: implementado; ejecucion condicionada a runner fisico.

## Regla de verdad

`physical_execution.performed` solo puede ser `true` en el certificado compuesto
cuando se verifican simultaneamente:

- bundle host de red y sincronizacion;
- bundle de placa con `status=certified` y `physical_board=true`;
- certificado `vfio_physical_passthrough`;
- DMA generado por el dispositivo, reset y recuperacion;
- grupo IOMMU aislado, IOVA y remapeo de interrupciones;
- MSI-X fisica entregada por `eventfd`;
- identidad BDF/vendor/device coincidente;
- PCAP o PCAPNG Ethernet con marcadores TX y RX y SHA-256 fijado.
- atestacion HMAC-SHA256 externa de al menos 32 bytes.

No son evidencia suficiente los contadores de un driver hospedado, MMIO
trazado, TAP, loopback, una interrupcion inyectada por software ni QEMU.

## Captura

El parser interno valida PCAP clasico y PCAPNG, longitudes de bloques, tipo de
enlace Ethernet, VLAN/QinQ, MAC origen/destino, EtherType y payload marcador.
Ambas direcciones deben aparecer el numero minimo de veces declarado.

El EtherType recomendado para el laboratorio es el experimental `0x88b5`.

## Retencion

El bundle copia como archivos regulares, sin symlinks:

- host certificate/report;
- board bundle completo y sus logs;
- certificado VFIO;
- captura;
- contrato de captura;
- informe y certificado compuesto.

El verificador vuelve a ejecutar los tres verificadores componentes, recalcula
la captura y compara identidad, contadores y hashes con el informe sellado.
Tambien recalcula el HMAC; el secreto no se incluye en el bundle ni se entrega
a los comandos de placa.

## Host actual

El host Windows observado dispone de una Intel BE200 activa y una Realtek
RTL8125 desconectada. Se midio trafico real mediante el driver Windows, pero no
hay privilegios elevados, captura PCAP ni VFIO/IOMMU exclusivo. Por tanto ese
resultado no se convierte en certificado fisico nC.
