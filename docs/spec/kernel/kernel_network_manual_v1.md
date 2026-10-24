# Red kernel manual v1

Estado normativo: activo.

La red kernel no reserva memoria, no crea hilos y no programa timers de forma
oculta. El programador entrega anillos DMA, buffers de paquetes, tablas de
sockets, almacenamiento de reensamblado y reloj monotono.

## NIC

`ManualNic` materializa reset, anillos RX/TX, doorbells, ownership de
descriptores, checksum offload, completion y recovery. El catalogo PCI incluye
familias historicas y actuales: NE2000, PCnet, Tulip, 3C905, RTL8139, RTL8169,
E1000/E1000e, igb, ixgbe, i40e, NetXtreme, ConnectX y VirtIO.

Los perfiles comparten `RegisterIo`; pueden ejecutarse sobre MMIO volatil real o
sobre un bus trazable. Una identidad no catalogada puede usar el mismo contrato
raw con offsets aportados por el driver. No se simula exito fisico cuando no hay
hardware conectado.

## Enlace y red

- Ethernet II, padding, VLAN 802.1Q y QinQ;
- EtherTypes actuales y heredados, incluidos IPv4, ARP/RARP, AppleTalk, IPX,
  IPv6, MPLS, PPPoE, EAPOL y LLDP;
- ARP request/reply, cache, estados y expiracion;
- rutas IPv4/IPv6 por prefijo mas largo y metrica;
- IPv4 con opciones, checksum, DF/MF, fragmentacion y reensamblado;
- IPv6 con traffic class, flow label, extensiones, fragmentacion, reensamblado
  e ICMPv6 Neighbor Solicitation;
- UDP IPv4/IPv6 con pseudo-header y regla de checksum propia de cada familia;
- TCP con estados, MSS, window scaling, SACK-permitted, timestamps, ventanas,
  retransmision, backoff, slow start, congestion avoidance y cierre;
- sockets UDP/TCP/raw, buffers circulares y timers caller-owned.

ESP cifrado se reconoce y se entrega a una capa IPsec explicita; esta pila no
descifra trafico sin claves ni politica aportadas por el programador.

## Fallos

Checksums incorrectos, longitudes truncadas, solapamiento de fragmentos,
descriptores ocupados, puertos duplicados, tablas llenas y transiciones TCP
invalidas producen errores tipados. No existe recuperacion o asignacion oculta.

## Certificacion

```text
ncc machine kernel-system network-sync --bundle target/evidence/kernel-network-sync
ncc machine kernel-system verify-network-sync target/evidence/kernel-network-sync
```

El certificado host valida bytes, estados y secuencias. La certificacion de una
NIC fisica exige un runner de placa con DMA, IRQ/MSI-X, IOMMU y logs retenidos.
