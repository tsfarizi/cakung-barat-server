#let surat_pernyataan(
  pengisi: (
    nama: "........................................",
    nik: "........................................",
    ttl: "........................................",
    jk: "........................................",
    agama: "........................................",
    pekerjaan: "........................................",
    alamat: "........................................",
    telp: "........................................",
  ),
  subjek: (
    nama: "........................................",
    nik: "........................................",
    ttl: "........................................",
    jk: "........................................",
    agama: "........................................",
    pekerjaan: "........................................",
    alamat: "........................................",
    hubungan: "........................................",
  ),
  meta: (
    opsi_sendiri: true,
    kelurahan: "........................................",
    tanggal: ".................... 2025",
  ),
) = {
  set page(paper: "a4", margin: 2.5cm)
  set text(font: "Times New Roman", size: 12pt)
  set par(justify: true, leading: 0.65em)

  let field(label, isi) = {
    grid(
      columns: (160pt, 10pt, 1fr),
      gutter: 0.6em,
      label, [:], isi,
    )
  }

  align(center)[
    #text(weight: "bold", size: 14pt)[SURAT PERNYATAAN TIDAK MAMPU]
  ]

  [Yang bertanda tangan dibawah ini:]

  field([Nama], pengisi.nama)
  field([NIK], pengisi.nik)
  field([Tempat & Tgl Lahir], pengisi.ttl)
  field([Jenis Kelamin], pengisi.jk)
  field([Agama], pengisi.agama)
  field([Pekerjaan], pengisi.pekerjaan)
  field([Alamat], pengisi.alamat)
  field([No. Telp / HP], pengisi.telp)

  [
    Menyatakan bahwa benar saya BERASAL DARI KELUARGA TIDAK MAMPU / TIDAK MAMPU SECARA FINANSIAL* dan bermaksud mengurus keperluan Surat Keterangan Tidak Mampu atas*
    #(if meta.opsi_sendiri { underline[diri saya sendiri] } else { [diri saya sendiri] }) /
    #(
      if not meta.opsi_sendiri { underline[*atas nama tersebut dibawah ini*] } else {
        [*atas nama tersebut dibawah ini*]
      }
    )
  ]

  field([Nama], subjek.nama)
  field([NIK (bila ada)], subjek.nik)
  field([Tempat & Tgl Lahir], subjek.ttl)
  field([Jenis Kelamin], subjek.jk)
  field([Agama], subjek.agama)
  field([Pekerjaan], subjek.pekerjaan)
  field([Alamat], subjek.alamat)
  field([Hubungan Keluarga], subjek.hubungan)

  [pada satuan pelaksana PTSP Kelurahan #meta.kelurahan]

  [Demikian surat pernyataan ini dibuat dengan sebenarnya dan apabila dikemudian hari terbukti surat pernyataan ini tidak benar dan/atau terjadi penyalahgunaan terkait layanan perizinan dan non perizinan yang diterbitkan maka saya bersedia dituntut sesuai dengan peraturan perundang-undangan yang berlaku dan dokumen yang telah diterbitkan dapat dibatalkan atau batal demi hukum.]

  text(size: 10pt)[*) Coret yang tidak perlu*]

  grid(
    columns: (1fr, 1fr),
    [],
    [
      Jakarta, #meta.tanggal \
      Yang membuat pernyataan,
      #v(1.5cm)
      #align(center)[
        #rect(width: 60pt, height: 40pt, stroke: 0.5pt + gray)[
          #set align(center + horizon)
          #text(size: 8pt)[materai\ Rp. 10.000]
        ]
      ]
      ( #pengisi.nama )
    ],
  )
}

#surat_pernyataan()
