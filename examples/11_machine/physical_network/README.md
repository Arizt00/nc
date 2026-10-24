# Certificacion fisica de red kernel

Esta carpeta no contiene una captura sintetica positiva. El contrato se rellena
despues de ejecutar una NIC PCI bajo VFIO en un runner fisico y capturar desde
un peer Ethernet independiente.

El ejercicio fisico debe:

1. arrancar la imagen nC en una placa identificada y producir un board bundle;
2. ligar la NIC seleccionada a `vfio-pci` dentro de un grupo IOMMU aislado;
3. mapear IOVA, programar DMA y recibir una MSI-X generada por el dispositivo;
4. transmitir `NCC-PHYSICAL-TX` con EtherType `0x88b5`;
5. recibir desde el peer `NCC-PHYSICAL-RX`;
6. conservar PCAP/PCAPNG, certificado VFIO, logs y hashes;
7. resetear, desmontar IOVA y restaurar el dispositivo.

Certificacion compuesta:

```text
ncc machine kernel-system network-sync-physical \
  --host-bundle host-network-sync \
  --board-bundle board-bundle \
  --vfio-certificate vfio-physical.json \
  --capture wire.pcapng \
  --capture-contract capture-contract.json \
  --bundle network-physical \
  --signing-key-env NCC_NETWORK_PHYSICAL_HMAC_KEY

ncc machine kernel-system verify-network-sync-physical network-physical \
  --signing-key-env NCC_NETWORK_PHYSICAL_HMAC_KEY
```

Contadores del driver hospedado, loopback, TAP, QEMU, una captura unidireccional
o un certificado `vfio_control_plane` se rechazan.

La clave HMAC debe contener al menos 32 bytes. Solo se lee desde el entorno; no
se persiste ni se hereda por los comandos de placa.
