#let surat_pernyataan_kpr(
  data: (
    nama: "........................................",
    nik: "........................................",
    ttl: "........................................",
    jk: "........................................",
    agama: "........................................",
    pekerjaan: "........................................",
    alamat: "........................................",
    telp: "........................................",
  ),
  meta: (
    kelurahan: "........................................",
    bank_tujuan: "........................................",
    tanggal: ".................... 2025",
  ),
) = {
  set page(paper: "a4", margin: (x: 2.5cm, y: 1.5cm))
  set text(font: "Times New Roman", size: 11pt)
  set par(justify: true, leading: 0.6em)

  let field(label, isi) = {
    grid(
      columns: (160pt, 10pt, 1fr),
      gutter: 0.5em,
      label, [:], isi,
    )
  }

  align(center)[
    #text(weight: "bold", size: 12pt)[SURAT PERNYATAAN] \
    #text(weight: "bold", size: 12pt)[BELUM MEMILIKI RUMAH]
  ]

  v(1em)
  [Yang bertanda tangan dibawah ini:]
  v(0.5em)

  pad(left: 2em)[
    #field([Nama], data.nama)
    #field([NIK], data.nik)
    #field([Tempat & Tgl Lahir], data.ttl)
    #field([Jenis Kelamin], data.jk)
    #field([Agama], data.agama)
    #field([Pekerjaan], data.pekerjaan)
    #field([Alamat], data.alamat)
    #field([No. Telp / HP], data.telp)
  ]

  v(1em)
  [
    menyatakan bahwa benar saya belum memiliki rumah dan bermaksud mengurus keperluan Surat Pengantar KPR pada Unit pelaksana PTSP Kelurahan #meta.kelurahan untuk mengurus permohonan pengajuan KPR di #meta.bank_tujuan.
  ]

  v(1em)
  [
    Demikian surat pernyataan ini dibuat dengan sebenarnya dan apabila dikemudian hari terbukti surat pernyataan ini tidak benar dan/atau terjadi penyalahgunaan terkait layanan perizinan dan non perizinan yang diterbitkan maka saya bersedia dituntut sesuai dengan peraturan perundang-undangan yang berlaku dan dokumen yang telah diterbitkan dapat dibatalkan atau batal demi hukum.
  ]

  v(2em)
  grid(
    columns: (1fr, 1fr),
    [],
    [
      Jakarta, #meta.tanggal \
      Yang membuat pernyataan,
      #v(0.8cm)
      #align(center)[
        #rect(width: 55pt, height: 35pt, stroke: 0.5pt + gray)[
          #set align(center + horizon)
          #text(size: 7pt)[materai\ Rp. 10.000]
        ]
      ]
      #v(0.8cm)
      ( #data.nama )
    ],
  )
}

#surat_pernyataan_kpr()
