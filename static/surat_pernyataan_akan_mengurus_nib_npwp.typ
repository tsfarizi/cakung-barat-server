#let surat_pernyataan_nib_npwp(
  data: (
    nama: "........................................",
    nik: "........................................",
    jabatan: "........................................",
    bidang_usaha: "........................................",
    kegiatan_usaha: "........................................",
    jenis_usaha: "........................................",
    alamat_usaha: "........................................",
  ),
  meta: (
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
    #text(weight: "bold", size: 12pt)[SURAT PERNYATAAN AKAN MENGURUS NIB & NPWP] \
    #text(size: 10pt)[(NOMOR INDUK BERUSAHA & NOMOR POKOK WAJIB PAJAK)]
  ]

  v(1em)
  [Saya yang bertanda tangan di bawah ini:]
  v(0.5em)

  pad(left: 2em)[
    #field([Nama], data.nama)
    #field([NIK], data.nik)
    #field([Jabatan], data.jabatan)
    #field([Bidang Usaha], data.bidang_usaha)
    #field([Kegiatan Usaha], data.kegiatan_usaha)
    #field([Jenis Usaha], data.jenis_usaha)
    #field([Alamat Usaha], data.alamat_usaha)
  ]

  v(1em)
  [Menyatakan sebagai berikut:]
  v(0.5em)

  enum(indent: 2em, tight: false)[
    Sampai saat ini belum memiliki NPWP & NIB serta berkomitmen mengurus pembuatan NPWP & NIB dalam waktu paling lambat 3 (tiga) bulan sejak pernyataan ini di buat.
  ][
    Siap dan sanggup menerima surat peringatan serta bersedia mendapatkan sanksi sesuai dengan perundang-undangan apabila dalam waktu yang tersebut diatas terbukti belum memiliki NPWP & NIB.
  ]

  v(1em)
  [Demikian surat pernyataan ini dibuat dengan sebenar benarnya]

  v(2em)
  grid(
    columns: (1fr, 1fr),
    [],
    [
      Jakarta, #meta.tanggal \
      Yang menyatakan,
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

#surat_pernyataan_nib_npwp()
