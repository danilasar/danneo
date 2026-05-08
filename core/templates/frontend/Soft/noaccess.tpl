<div class="site-title"><img src="{site_url}/temp/{site_temp}/images/icon/act.gif" alt="" /> &nbsp{ou_title}</div>
<div class="form-area">
 <form action="{site_url}/index.php?dn=user" method="post">
  <fieldset>
   <legend>{login}</legend>
    <input class="width" name="login" size="25" type="text" maxlength="{maxname}" />
  </fieldset>
  <fieldset>
   <legend>{pass}</legend>
    <input class="width" name="passw" size="25" type="password" maxlength="{maxpass}" />
  </fieldset>
  <div class="form-area-apart ac">
   <input name="re" value="login" type="hidden" />
   <input name="to" value="check" type="hidden" />
   {redirect}
   <button type="submit" class="sub">{enter}</button>
  </div>
  <div class="form-area-apart">
   <p class="user norm"><a rel="nofollow" href="{site_url}/{linklost}" title="{send_pass}">{send_pass}</a></p>
   <p class="user norm"><a rel="nofollow" href="{site_url}/{linkreg}" title="{registr}">{registr}</a></p>
  </div>
 </form>
</div>
